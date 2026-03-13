//! End-to-end instrument-MIDI integration tests.
//!
//! These tests exercise the full pipeline: construct NoteEvents from a MidiClip,
//! feed them through each instrument's process() method, and verify non-silent
//! audio output.

use shruti_dsp::AudioBuffer;
use shruti_instruments::DrumMachine;
use shruti_instruments::instrument::InstrumentNode;
use shruti_instruments::sampler::{SampleZone, Sampler};
use shruti_instruments::synth::SubtractiveSynth;
use shruti_session::midi::{ControlChange, MidiClip, NoteEvent};

const SAMPLE_RATE: f32 = 44100.0;
const BLOCK_SIZE: u32 = 512;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a sine-wave sample buffer of the given length.
fn make_sine(len: usize, freq: f32, sr: f32) -> Vec<f32> {
    (0..len)
        .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin())
        .collect()
}

/// Check whether a buffer contains any sample with absolute value above `threshold`.
fn has_nonzero(buf: &AudioBuffer, threshold: f32) -> bool {
    for frame in 0..buf.frames() {
        for ch in 0..buf.channels() {
            if buf.get(frame, ch).abs() > threshold {
                return true;
            }
        }
    }
    false
}

/// Check whether a buffer is entirely silent (all samples below `threshold`).
fn is_silent(buf: &AudioBuffer, threshold: f32) -> bool {
    !has_nonzero(buf, threshold)
}

/// Build NoteEvents from a MidiClip using its note_ons_at helper.
/// Collects note-on events that start at `frame` (absolute position).
fn collect_note_ons(clip: &MidiClip, frame: u64) -> Vec<NoteEvent> {
    clip.note_ons_at(frame).into_iter().cloned().collect()
}

// ---------------------------------------------------------------------------
// 1. Synth plays MIDI clip
// ---------------------------------------------------------------------------

#[test]
fn synth_plays_midi_clip() {
    let mut clip = MidiClip::new("Synth Test", 0, 48000);
    clip.add_note(0, 4800, 60, 100, 0); // C4
    clip.add_note(0, 4800, 64, 90, 0); // E4
    clip.add_note(0, 4800, 67, 80, 0); // G4

    // Collect note-on events at the clip start (frame 0).
    let note_events = collect_note_ons(&clip, 0);
    assert_eq!(note_events.len(), 3);

    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    let mut buf = AudioBuffer::new(2, BLOCK_SIZE);
    synth.process(&note_events, &[], &mut buf);

    assert!(
        has_nonzero(&buf, 0.001),
        "SubtractiveSynth should produce non-silent output when fed NoteEvents from a MidiClip"
    );
}

// ---------------------------------------------------------------------------
// 2. DrumMachine plays MIDI clip
// ---------------------------------------------------------------------------

#[test]
fn drum_machine_plays_midi_clip() {
    let mut clip = MidiClip::new("Drums", 0, 48000);
    clip.add_note(0, 2400, 36, 127, 9); // Bass Drum (pad 0, note 36)
    clip.add_note(0, 2400, 38, 110, 9); // Snare (pad 2, note 38)
    clip.add_note(0, 2400, 42, 100, 9); // Closed HH (pad 6, note 42)

    let note_events = collect_note_ons(&clip, 0);
    assert_eq!(note_events.len(), 3);

    let mut dm = DrumMachine::new(SAMPLE_RATE);
    // Load sine samples onto the target pads.
    let sample = make_sine(4410, 200.0, SAMPLE_RATE);
    dm.pads[0].load_sample(sample.clone(), SAMPLE_RATE as u32); // note 36
    dm.pads[2].load_sample(sample.clone(), SAMPLE_RATE as u32); // note 38
    dm.pads[6].load_sample(sample, SAMPLE_RATE as u32); // note 42

    let mut buf = AudioBuffer::new(2, BLOCK_SIZE);
    dm.process(&note_events, &[], &mut buf);

    assert!(
        has_nonzero(&buf, 0.001),
        "DrumMachine should produce audio when playing drum note events from a MidiClip"
    );
}

// ---------------------------------------------------------------------------
// 3. Sampler plays MIDI clip
// ---------------------------------------------------------------------------

#[test]
fn sampler_plays_midi_clip() {
    let mut clip = MidiClip::new("Sampler Test", 0, 48000);
    clip.add_note(0, 4800, 60, 100, 0);

    let note_events = collect_note_ons(&clip, 0);

    let mut sampler = Sampler::new(SAMPLE_RATE);
    let zone = SampleZone::new(
        "Piano",
        60,
        make_sine(44100, 440.0, SAMPLE_RATE),
        SAMPLE_RATE as u32,
    );
    sampler.add_zone(zone);

    let mut buf = AudioBuffer::new(2, BLOCK_SIZE);
    sampler.process(&note_events, &[], &mut buf);

    assert!(
        has_nonzero(&buf, 0.001),
        "Sampler should produce audio when processing NoteEvents from a MidiClip"
    );
}

// ---------------------------------------------------------------------------
// 4. CC changes affect instrument
//
// The InstrumentNode::process() signature accepts &[ControlChange], but the
// current implementations (SubtractiveSynth, DrumMachine, Sampler) prefix the
// parameter with `_` -- they ignore CC events inside process(). We still verify
// that sending CC events does NOT crash, and demonstrate that parameter changes
// via params_mut() DO take effect on the audio output.
// ---------------------------------------------------------------------------

#[test]
fn cc_events_do_not_crash_and_params_affect_output() {
    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    synth.note_on(69, 127, 0);

    // Render a baseline block.
    let mut buf_before = AudioBuffer::new(2, BLOCK_SIZE);
    synth.process(&[], &[], &mut buf_before);

    // Reset, change volume to 0 via params, render again.
    synth.reset();
    synth.params_mut().iter_mut().for_each(|p| {
        if p.name == "Volume" {
            p.set(0.0);
        }
    });
    synth.note_on(69, 127, 0);
    let mut buf_after = AudioBuffer::new(2, BLOCK_SIZE);
    synth.process(&[], &[], &mut buf_after);

    assert!(
        has_nonzero(&buf_before, 0.001),
        "Synth at default volume should produce sound"
    );
    assert!(
        is_silent(&buf_after, 0.001),
        "Synth at volume=0 should be silent"
    );

    // Also verify that passing CC events does not panic.
    let cc_events = vec![
        ControlChange {
            position: 0,
            controller: 1, // Mod wheel
            value: 64,
            channel: 0,
        },
        ControlChange {
            position: 100,
            controller: 74, // Filter cutoff (common CC)
            value: 32,
            channel: 0,
        },
    ];
    synth.reset();
    synth.params_mut().iter_mut().for_each(|p| {
        if p.name == "Volume" {
            p.set(0.8);
        }
    });
    synth.note_on(69, 127, 0);
    let mut buf_cc = AudioBuffer::new(2, BLOCK_SIZE);
    synth.process(&[], &cc_events, &mut buf_cc);
    // No panic means success; the CC path is a no-op currently but should be safe.
}

// ---------------------------------------------------------------------------
// 5. Note off stops sound
// ---------------------------------------------------------------------------

#[test]
fn note_off_stops_sound() {
    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);

    // Use very short release so the sound dies quickly after note-off.
    synth
        .params_mut()
        .iter_mut()
        .for_each(|p| match p.name.as_str() {
            "Release" => p.set(0.005), // 5ms release
            "Attack" => p.set(0.001),
            _ => {}
        });

    // Play a note.
    synth.note_on(60, 127, 0);
    let mut buf = AudioBuffer::new(2, BLOCK_SIZE);
    synth.process(&[], &[], &mut buf);
    assert!(has_nonzero(&buf, 0.001), "Note should produce sound");

    // Send note-off.
    synth.note_off(60, 0);

    // Process several blocks to let the release envelope die out.
    // With 5ms release at 44100 Hz that is ~220 samples. A few blocks should suffice.
    let mut went_silent = false;
    for _ in 0..20 {
        let mut buf2 = AudioBuffer::new(2, BLOCK_SIZE);
        synth.process(&[], &[], &mut buf2);
        if is_silent(&buf2, 0.0001) {
            went_silent = true;
            break;
        }
    }

    assert!(
        went_silent,
        "Sound should decay to silence after note_off with short release"
    );
}

// ---------------------------------------------------------------------------
// 6. Multiple simultaneous notes (polyphony)
// ---------------------------------------------------------------------------

#[test]
fn multiple_simultaneous_notes() {
    let mut clip = MidiClip::new("Chord", 0, 48000);
    clip.add_note(0, 4800, 60, 100, 0); // C4
    clip.add_note(0, 4800, 64, 100, 0); // E4
    clip.add_note(0, 4800, 67, 100, 0); // G4
    clip.add_note(0, 4800, 72, 100, 0); // C5

    let note_events = collect_note_ons(&clip, 0);
    assert_eq!(note_events.len(), 4);

    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    let mut buf = AudioBuffer::new(2, BLOCK_SIZE);
    synth.process(&note_events, &[], &mut buf);

    // All four voices should be active.
    assert_eq!(
        synth.active_voices(),
        4,
        "Should have 4 active voices for a 4-note chord"
    );
    assert!(has_nonzero(&buf, 0.001), "Chord should produce audio");

    // The combined output should be louder than a single note (roughly).
    let mut rms_chord = 0.0_f64;
    for frame in 0..BLOCK_SIZE {
        let s = buf.get(frame, 0) as f64;
        rms_chord += s * s;
    }
    rms_chord = (rms_chord / BLOCK_SIZE as f64).sqrt();

    // Now render a single note for comparison.
    let mut synth_single = SubtractiveSynth::new(SAMPLE_RATE);
    let single_events = vec![NoteEvent {
        position: 0,
        duration: 4800,
        note: 60,
        velocity: 100,
        channel: 0,
    }];
    let mut buf_single = AudioBuffer::new(2, BLOCK_SIZE);
    synth_single.process(&single_events, &[], &mut buf_single);

    let mut rms_single = 0.0_f64;
    for frame in 0..BLOCK_SIZE {
        let s = buf_single.get(frame, 0) as f64;
        rms_single += s * s;
    }
    rms_single = (rms_single / BLOCK_SIZE as f64).sqrt();

    assert!(
        rms_chord > rms_single,
        "Chord RMS ({rms_chord}) should exceed single-note RMS ({rms_single})"
    );
}

// ---------------------------------------------------------------------------
// 7. Empty clip produces silence
// ---------------------------------------------------------------------------

#[test]
fn empty_clip_produces_silence() {
    let clip = MidiClip::new("Empty", 0, 48000);
    let note_events = collect_note_ons(&clip, 0);
    assert!(note_events.is_empty());

    // SubtractiveSynth
    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    let mut buf_synth = AudioBuffer::new(2, BLOCK_SIZE);
    synth.process(&note_events, &[], &mut buf_synth);
    assert!(
        is_silent(&buf_synth, 0.0),
        "Synth with empty clip must be silent"
    );

    // DrumMachine
    let mut dm = DrumMachine::new(SAMPLE_RATE);
    dm.pads[0].load_sample(make_sine(4410, 200.0, SAMPLE_RATE), SAMPLE_RATE as u32);
    let mut buf_dm = AudioBuffer::new(2, BLOCK_SIZE);
    dm.process(&note_events, &[], &mut buf_dm);
    assert!(
        is_silent(&buf_dm, 0.0),
        "DrumMachine with empty clip must be silent"
    );

    // Sampler
    let mut sampler = Sampler::new(SAMPLE_RATE);
    sampler.add_zone(SampleZone::new(
        "Test",
        60,
        make_sine(4410, 440.0, SAMPLE_RATE),
        SAMPLE_RATE as u32,
    ));
    let mut buf_sampler = AudioBuffer::new(2, BLOCK_SIZE);
    sampler.process(&note_events, &[], &mut buf_sampler);
    assert!(
        is_silent(&buf_sampler, 0.0),
        "Sampler with empty clip must be silent"
    );
}
