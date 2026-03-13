//! End-to-end integration tests for Shruti DAW.
//!
//! These tests exercise cross-crate workflows that unit tests miss,
//! verifying that the session, DSP, and instrument crates work together.

use std::f32::consts::PI;

use shruti_dsp::AudioBuffer;
use shruti_dsp::effects::eq::{EqBand, FilterType};
use shruti_dsp::effects::{Compressor, Delay, ParametricEq, Reverb};
use shruti_dsp::format::AudioFormat;
use shruti_dsp::io::{
    BitDepth, ExportConfig, ExportFormat, read_audio_file, write_audio_file, write_wav_file,
};
use shruti_instruments::drum_machine::DrumMachine;
use shruti_instruments::instrument::InstrumentNode;
use shruti_instruments::preset::InstrumentPreset;
use shruti_instruments::routing::{MidiRoute, VelocityCurve};
use shruti_instruments::sampler::{SampleZone, Sampler};
use shruti_instruments::synth::SubtractiveSynth;
use shruti_session::automation::{AutomationLane, AutomationPoint, AutomationTarget, CurveType};
use shruti_session::edit::EditCommand;
use shruti_session::midi::{MidiClip, NoteEvent};
use shruti_session::region::Region;
use shruti_session::session::Session;
use shruti_session::store::SessionStore;
use shruti_session::track::SendPosition;
use shruti_session::undo::UndoManager;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a mono sine wave at the given frequency.
fn generate_sine(freq: f32, sample_rate: f32, frames: usize, amplitude: f32) -> Vec<f32> {
    (0..frames)
        .map(|i| (2.0 * PI * freq * i as f32 / sample_rate).sin() * amplitude)
        .collect()
}

/// Compute RMS of a single channel in a buffer.
fn rms(buf: &AudioBuffer, channel: u16, frames: usize) -> f32 {
    let sum: f32 = (0..frames)
        .map(|i| buf.get(i as u32, channel).powi(2))
        .sum();
    (sum / frames as f32).sqrt()
}

/// Check that a buffer has non-silent content (any sample above threshold).
fn has_audio(buf: &AudioBuffer, threshold: f32) -> bool {
    buf.as_interleaved().iter().any(|&s| s.abs() > threshold)
}

// ===========================================================================
// 1. Full audio pipeline
// ===========================================================================

#[test]
fn full_audio_pipeline_renders_sine_through_timeline() {
    let sample_rate = 48000u32;
    let buffer_size = 1024u32;

    // Create session with two audio tracks.
    let mut session = Session::new("Pipeline Test", sample_rate, buffer_size);
    let track1_id = session.add_audio_track("Sine 440");
    let track2_id = session.add_audio_track("Sine 880");

    // Generate sine waves and insert into the audio pool.
    let frames = buffer_size as usize;
    let sine_440: Vec<f32> = generate_sine(440.0, sample_rate as f32, frames, 0.5)
        .iter()
        .flat_map(|&s| vec![s, s]) // stereo
        .collect();
    let sine_880: Vec<f32> = generate_sine(880.0, sample_rate as f32, frames, 0.3)
        .iter()
        .flat_map(|&s| vec![s, s])
        .collect();

    session.audio_pool.insert(
        "sine440.wav".into(),
        AudioBuffer::from_interleaved(sine_440.clone(), 2),
    );
    session.audio_pool.insert(
        "sine880.wav".into(),
        AudioBuffer::from_interleaved(sine_880.clone(), 2),
    );

    // Add regions to tracks.
    let track1 = session.track_mut(track1_id).unwrap();
    track1.add_region(Region::new("sine440.wav".into(), 0, 0, buffer_size as u64));

    let track2 = session.track_mut(track2_id).unwrap();
    track2.add_region(Region::new("sine880.wav".into(), 0, 0, buffer_size as u64));

    // Render through the timeline.
    let mut output = AudioBuffer::new(2, buffer_size);
    let timeline = session.timeline.as_mut().unwrap();
    timeline.render(
        &session.tracks,
        &session.transport,
        &session.audio_pool,
        &mut output,
    );

    // Verify the output has audio (the mix of both sines).
    assert!(
        has_audio(&output, 0.01),
        "Output should contain mixed audio from both tracks"
    );

    // Verify the output is the sum of both tracks.
    // At frame 0 both sines start at 0, so check a few frames in.
    for frame in 10..20 {
        let expected_l = sine_440[frame * 2] + sine_880[frame * 2];
        let actual = output.get(frame as u32, 0);
        assert!(
            (actual - expected_l).abs() < 1e-5,
            "Frame {frame}: expected {expected_l}, got {actual}"
        );
    }
}

#[test]
fn full_audio_pipeline_muted_track_excluded() {
    let sample_rate = 48000u32;
    let buffer_size = 512u32;

    let mut session = Session::new("Mute Test", sample_rate, buffer_size);
    let track_id = session.add_audio_track("Sine");

    let frames = buffer_size as usize;
    let sine: Vec<f32> = generate_sine(440.0, sample_rate as f32, frames, 0.5)
        .iter()
        .flat_map(|&s| vec![s, s])
        .collect();
    session
        .audio_pool
        .insert("sine.wav".into(), AudioBuffer::from_interleaved(sine, 2));

    let track = session.track_mut(track_id).unwrap();
    track.add_region(Region::new("sine.wav".into(), 0, 0, buffer_size as u64));
    track.muted = true;

    let mut output = AudioBuffer::new(2, buffer_size);
    let timeline = session.timeline.as_mut().unwrap();
    timeline.render(
        &session.tracks,
        &session.transport,
        &session.audio_pool,
        &mut output,
    );

    assert!(
        !has_audio(&output, 1e-6),
        "Muted track should produce silence"
    );
}

// ===========================================================================
// 2. Instrument pipeline (SubtractiveSynth)
// ===========================================================================

#[test]
fn instrument_pipeline_synth_produces_audio_from_midi() {
    let sample_rate = 48000.0f32;
    let buffer_size = 2048u32;

    // Create a session with an instrument track.
    let mut session = Session::new("Synth Test", sample_rate as u32, buffer_size);
    let _track_id = session.add_instrument_track("Lead Synth", Some("SubtractiveSynth".into()));

    // Create MIDI clip with a chord (C major: C4, E4, G4).
    let mut clip = MidiClip::new("Chord", 0, buffer_size as u64);
    clip.add_note(0, buffer_size as u64, 60, 100, 0); // C4
    clip.add_note(0, buffer_size as u64, 64, 90, 0); // E4
    clip.add_note(0, buffer_size as u64, 67, 80, 0); // G4

    // Create synth and send note-on events.
    let mut synth = SubtractiveSynth::new(sample_rate);

    // Trigger notes from the clip.
    for note in &clip.notes {
        synth.note_on(note.note, note.velocity, note.channel);
    }

    // Process audio.
    let mut output = AudioBuffer::new(2, buffer_size);
    synth.process(&[], &[], &mut output);

    // Verify non-silent output.
    assert!(
        has_audio(&output, 0.001),
        "Synth should produce audio when notes are triggered"
    );

    // Verify multiple voices are active.
    assert_eq!(
        synth.active_voices(),
        3,
        "Three notes should produce three voices"
    );

    // Verify the output is within a reasonable range (no clipping).
    let peak = output
        .as_interleaved()
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    assert!(peak < 3.0, "Peak should be reasonable, got {peak}");
}

#[test]
fn instrument_pipeline_synth_note_off_decays() {
    let sample_rate = 48000.0f32;
    let buffer_size = 4096u32;

    let mut synth = SubtractiveSynth::new(sample_rate);
    synth.note_on(60, 100, 0);

    // Process one buffer with the note on.
    let mut buf1 = AudioBuffer::new(2, buffer_size);
    synth.process(&[], &[], &mut buf1);
    let rms_on = rms(&buf1, 0, buffer_size as usize);

    // Send note off.
    synth.note_off(60, 0);

    // Process several buffers to allow the release envelope to decay.
    let mut last_rms = rms_on;
    for _ in 0..10 {
        let mut buf = AudioBuffer::new(2, buffer_size);
        synth.process(&[], &[], &mut buf);
        let current_rms = rms(&buf, 0, buffer_size as usize);
        // After note off, RMS should eventually decrease.
        last_rms = current_rms;
    }

    assert!(
        last_rms < rms_on * 0.5,
        "After note off and release, RMS should decrease: on={rms_on}, last={last_rms}"
    );
}

// ===========================================================================
// 3. Drum machine pipeline
// ===========================================================================

#[test]
fn drum_machine_pad_playback_from_midi() {
    let sample_rate = 48000.0f32;
    let buffer_size = 2048u32;

    let mut dm = DrumMachine::new(sample_rate);

    // Load a synthetic kick sample into pad 0 (MIDI note 36).
    let kick_samples: Vec<f32> = (0..4800)
        .map(|i| {
            let t = i as f32 / sample_rate;
            // Simple kick: decaying low sine.
            let freq = 60.0 + 200.0 * (-t * 30.0).exp();
            (2.0 * PI * freq * t).sin() * (-t * 10.0).exp()
        })
        .collect();
    dm.pads[0].load_sample(kick_samples.clone(), sample_rate as u32);

    // Load a hi-hat into pad 2 (MIDI note 38).
    let hat_samples: Vec<f32> = (0..2400)
        .map(|i| {
            let t = i as f32 / sample_rate;
            // Simple noise burst using wrapping arithmetic.
            let noise = ((i as u32).wrapping_mul(1103515245).wrapping_add(12345) as f32
                / u32::MAX as f32)
                * 2.0
                - 1.0;
            noise * (-t * 40.0).exp()
        })
        .collect();
    dm.pads[2].load_sample(hat_samples, sample_rate as u32);

    // Trigger pad 0 (kick) via MIDI note 36.
    dm.note_on(36, 127, 0);

    // Process one buffer.
    let mut output = AudioBuffer::new(2, buffer_size);
    dm.process(&[], &[], &mut output);

    // Verify output has audio.
    assert!(
        has_audio(&output, 0.001),
        "DrumMachine should produce audio when a pad with a loaded sample is triggered"
    );

    // Trigger pad 2 (hi-hat) via MIDI note 38.
    dm.note_on(38, 100, 0);
    let mut output2 = AudioBuffer::new(2, buffer_size);
    dm.process(&[], &[], &mut output2);

    assert!(
        has_audio(&output2, 0.001),
        "DrumMachine should produce audio from second pad"
    );
}

#[test]
fn drum_machine_empty_pad_is_silent() {
    let sample_rate = 48000.0f32;
    let buffer_size = 512u32;

    let mut dm = DrumMachine::new(sample_rate);

    // Trigger an empty pad (no sample loaded).
    dm.note_on(36, 127, 0);

    let mut output = AudioBuffer::new(2, buffer_size);
    dm.process(&[], &[], &mut output);

    assert!(
        !has_audio(&output, 1e-6),
        "Triggering an empty pad should produce silence"
    );
}

// ===========================================================================
// 4. Effects chain
// ===========================================================================

#[test]
fn effects_chain_eq_compressor_reverb_delay() {
    let sample_rate = 48000.0f32;
    let frames = 4800usize;

    // Generate a known 1kHz sine signal.
    let data = generate_sine(1000.0, sample_rate, frames, 0.5);
    let stereo: Vec<f32> = data.iter().flat_map(|&s| vec![s, s]).collect();
    let original = AudioBuffer::from_interleaved(stereo.clone(), 2);

    // --- Stage 1: EQ (12 dB boost at 1kHz) ---
    let mut eq = ParametricEq::new(sample_rate);
    eq.add_band(EqBand::new(FilterType::Peak, 1000.0, 12.0, 1.0));

    let mut buf_eq = AudioBuffer::from_interleaved(stereo.clone(), 2);
    eq.process(&mut buf_eq);

    let rms_before_eq = rms(&original, 0, frames);
    let rms_after_eq = rms(&buf_eq, 0, frames);
    assert!(
        rms_after_eq > rms_before_eq * 1.5,
        "EQ boost should increase RMS: before={rms_before_eq}, after={rms_after_eq}"
    );

    // --- Stage 2: Compressor ---
    let mut comp = Compressor::new(sample_rate);
    comp.threshold_db = -20.0;
    comp.ratio = 8.0;
    comp.attack = 0.001;
    comp.release = 0.05;
    comp.knee_db = 0.0;
    comp.makeup_db = 0.0;

    let mut buf_comp = buf_eq.clone();
    comp.process(&mut buf_comp);

    let rms_after_comp = rms(&buf_comp, 0, frames);
    // Compressor should reduce the boosted signal.
    assert!(
        rms_after_comp < rms_after_eq,
        "Compressor should reduce level: eq={rms_after_eq}, comp={rms_after_comp}"
    );

    // --- Stage 3: Reverb ---
    let mut reverb = Reverb::new(sample_rate);
    reverb.mix = 0.3;
    reverb.room_size = 0.5;
    reverb.update_parameters();

    let mut buf_reverb = buf_comp.clone();
    reverb.process(&mut buf_reverb);

    // Reverb modifies the signal (adds wet content).
    let diff_reverb: f32 = (0..frames)
        .map(|i| (buf_reverb.get(i as u32, 0) - buf_comp.get(i as u32, 0)).abs())
        .sum::<f32>()
        / frames as f32;
    assert!(
        diff_reverb > 0.001,
        "Reverb should modify the signal: avg diff={diff_reverb}"
    );

    // --- Stage 4: Delay ---
    let mut delay = Delay::new(sample_rate);
    delay.time = 0.01;
    delay.feedback = 0.3;
    delay.mix = 0.3;

    let mut buf_delay = buf_reverb.clone();
    delay.process(&mut buf_delay);

    // Delay modifies the signal.
    let diff_delay: f32 = (0..frames)
        .map(|i| (buf_delay.get(i as u32, 0) - buf_reverb.get(i as u32, 0)).abs())
        .sum::<f32>()
        / frames as f32;
    assert!(
        diff_delay > 0.001,
        "Delay should modify the signal: avg diff={diff_delay}"
    );

    // Verify the full chain output is finite and non-silent.
    assert!(
        has_audio(&buf_delay, 0.01),
        "Final output after full effects chain should be non-silent"
    );
    for i in 0..frames {
        assert!(
            buf_delay.get(i as u32, 0).is_finite(),
            "Output must be finite at frame {i}"
        );
    }
}

// ===========================================================================
// 5. Session persistence roundtrip
// ===========================================================================

#[test]
fn session_persistence_roundtrip_full() {
    let dir = tempfile::tempdir().unwrap();
    let session_path = dir.path().join("test_project.shruti");

    // Build a complex session.
    let mut session = Session::new("Persistence Test", 48000, 256);
    session.transport.bpm = 145.0;

    let audio1 = session.add_audio_track("Guitar");
    let audio2 = session.add_audio_track("Vocals");
    let _midi = session.add_midi_track("Keys MIDI");
    let bus = session.add_bus_track("FX Bus");
    let _inst = session.add_instrument_track("Synth Lead", Some("SubtractiveSynth".into()));
    let _dm = session.add_drum_machine_track("Drums", Some("808".into()));

    // Set track properties.
    session.track_mut(audio1).unwrap().gain = 0.75;
    session.track_mut(audio1).unwrap().pan = -0.3;
    session.track_mut(audio2).unwrap().gain = 0.9;

    // Add a region to audio1.
    let region = Region::new("guitar_take1.wav".into(), 0, 0, 48000);
    session.track_mut(audio1).unwrap().add_region(region);

    // Add a second region.
    let region2 = Region::new("guitar_take2.wav".into(), 48000, 0, 24000);
    session.track_mut(audio1).unwrap().add_region(region2);

    // Add a region to audio2.
    let region3 = Region::new("vocals.wav".into(), 0, 0, 96000);
    session.track_mut(audio2).unwrap().add_region(region3);

    // Add a send.
    session.add_send(audio1, bus, 0.5, SendPosition::PostFader);

    // Add automation.
    let mut auto_lane = AutomationLane::new(AutomationTarget::TrackGain);
    auto_lane.add_point(AutomationPoint {
        position: 0,
        value: 0.0,
        curve: CurveType::Linear,
    });
    auto_lane.add_point(AutomationPoint {
        position: 48000,
        value: 1.0,
        curve: CurveType::SCurve,
    });
    auto_lane.add_point(AutomationPoint {
        position: 96000,
        value: 0.5,
        curve: CurveType::Step,
    });
    session
        .track_mut(audio1)
        .unwrap()
        .automation
        .push(auto_lane);

    // Add groups.
    let group_id = session.add_group("Guitars");
    session.add_track_to_group(group_id, audio1);

    // Save.
    let store = SessionStore::create(&session_path, &session).unwrap();
    store
        .save_audio_pool(&session.audio_pool, session.sample_rate)
        .unwrap();

    // Load back.
    let (_store2, loaded) = SessionStore::open(&session_path).unwrap();

    // Verify all data matches.
    assert_eq!(loaded.name, "Persistence Test");
    assert_eq!(loaded.sample_rate, 48000);
    assert_eq!(loaded.buffer_size, 256);
    assert_eq!(loaded.transport.bpm, 145.0);

    // Track count: 6 user tracks + 1 master = 7.
    assert_eq!(loaded.tracks.len(), 7);

    // Audio track properties.
    let loaded_audio1 = loaded.track(audio1).unwrap();
    assert!((loaded_audio1.gain - 0.75).abs() < 1e-6);
    assert!((loaded_audio1.pan - -0.3).abs() < 1e-6);
    assert_eq!(loaded_audio1.regions.len(), 2);
    assert_eq!(loaded_audio1.regions[0].audio_file_id, "guitar_take1.wav");
    assert_eq!(loaded_audio1.regions[1].audio_file_id, "guitar_take2.wav");
    assert_eq!(loaded_audio1.regions[1].timeline_pos, 48000);

    // Sends.
    assert_eq!(loaded_audio1.sends.len(), 1);
    assert_eq!(loaded_audio1.sends[0].target, bus);
    assert!((loaded_audio1.sends[0].level - 0.5).abs() < 1e-6);

    // Automation.
    assert_eq!(loaded_audio1.automation.len(), 1);
    let loaded_auto = &loaded_audio1.automation[0];
    assert_eq!(loaded_auto.points.len(), 3);
    assert_eq!(loaded_auto.points[0].position, 0);
    assert!((loaded_auto.points[0].value - 0.0).abs() < 1e-6);
    assert_eq!(loaded_auto.points[1].curve, CurveType::SCurve);
    assert_eq!(loaded_auto.points[2].curve, CurveType::Step);

    // Groups.
    assert_eq!(loaded.groups.len(), 1);
    assert_eq!(loaded.groups[0].name, "Guitars");
    assert!(loaded.groups[0].contains(audio1));

    // Vocals track.
    let loaded_audio2 = loaded.track(audio2).unwrap();
    assert!((loaded_audio2.gain - 0.9).abs() < 1e-6);
    assert_eq!(loaded_audio2.regions.len(), 1);
}

// ===========================================================================
// 6. Undo/redo comprehensive
// ===========================================================================

#[test]
fn undo_redo_comprehensive_12_operations() {
    let mut session = Session::new("Undo Test", 48000, 256);
    let track_id = session.add_audio_track("Track 1");
    let track2_id = session.add_audio_track("Track 2");

    let mut undo = UndoManager::new(100);

    // Snapshot the original state.
    let original_gain = session.track(track_id).unwrap().gain;
    let original_pan = session.track(track_id).unwrap().pan;
    let original_region_count = session.track(track_id).unwrap().regions.len();

    // Operation 1: Add region.
    let region = Region::new("file1.wav".into(), 0, 0, 1000);
    let region_id = region.id;
    undo.execute(
        EditCommand::AddRegion {
            track_id,
            region: region.clone(),
        },
        &mut session,
    );
    assert_eq!(session.track(track_id).unwrap().regions.len(), 1);

    // Operation 2: Add another region.
    let region2 = Region::new("file2.wav".into(), 1000, 0, 2000);
    let region2_id = region2.id;
    undo.execute(
        EditCommand::AddRegion {
            track_id,
            region: region2.clone(),
        },
        &mut session,
    );
    assert_eq!(session.track(track_id).unwrap().regions.len(), 2);

    // Operation 3: Move region.
    undo.execute(
        EditCommand::MoveRegion {
            track_id,
            region_id,
            old_pos: 0,
            new_pos: 500,
        },
        &mut session,
    );
    assert_eq!(
        session
            .track(track_id)
            .unwrap()
            .region(region_id)
            .unwrap()
            .timeline_pos,
        500
    );

    // Operation 4: Set track gain.
    undo.execute(
        EditCommand::SetTrackGain {
            track_id,
            old_gain: original_gain,
            new_gain: 0.5,
        },
        &mut session,
    );
    assert!((session.track(track_id).unwrap().gain - 0.5).abs() < 1e-6);

    // Operation 5: Set track pan.
    undo.execute(
        EditCommand::SetTrackPan {
            track_id,
            old_pan: original_pan,
            new_pan: -0.7,
        },
        &mut session,
    );
    assert!((session.track(track_id).unwrap().pan - -0.7).abs() < 1e-6);

    // Operation 6: Toggle mute.
    undo.execute(EditCommand::ToggleTrackMute { track_id }, &mut session);
    assert!(session.track(track_id).unwrap().muted);

    // Operation 7: Toggle solo.
    undo.execute(EditCommand::ToggleTrackSolo { track_id }, &mut session);
    assert!(session.track(track_id).unwrap().solo);

    // Operation 8: Set region gain.
    undo.execute(
        EditCommand::SetRegionGain {
            track_id,
            region_id,
            old_gain: 1.0,
            new_gain: 0.3,
        },
        &mut session,
    );
    assert!(
        (session
            .track(track_id)
            .unwrap()
            .region(region_id)
            .unwrap()
            .gain
            - 0.3)
            .abs()
            < 1e-6
    );

    // Operation 9: Set fade in.
    undo.execute(
        EditCommand::SetFadeIn {
            track_id,
            region_id,
            old_fade: 0,
            new_fade: 100,
        },
        &mut session,
    );
    assert_eq!(
        session
            .track(track_id)
            .unwrap()
            .region(region_id)
            .unwrap()
            .fade_in,
        100
    );

    // Operation 10: Set fade out.
    undo.execute(
        EditCommand::SetFadeOut {
            track_id,
            region_id,
            old_fade: 0,
            new_fade: 200,
        },
        &mut session,
    );
    assert_eq!(
        session
            .track(track_id)
            .unwrap()
            .region(region_id)
            .unwrap()
            .fade_out,
        200
    );

    // Operation 11: Add another region to track 2.
    let region3 = Region::new("file3.wav".into(), 0, 0, 500);
    undo.execute(
        EditCommand::AddRegion {
            track_id: track2_id,
            region: region3,
        },
        &mut session,
    );
    assert_eq!(session.track(track2_id).unwrap().regions.len(), 1);

    // Operation 12: Remove region2.
    undo.execute(
        EditCommand::RemoveRegion {
            track_id,
            region_id: region2_id,
            region: None,
        },
        &mut session,
    );
    assert_eq!(session.track(track_id).unwrap().regions.len(), 1);

    // Verify 12 operations recorded.
    assert_eq!(undo.undo_count(), 12);

    // Snapshot edited state.
    let edited_track1_regions = session.track(track_id).unwrap().regions.len();
    let edited_gain = session.track(track_id).unwrap().gain;

    // Undo ALL 12 operations.
    for i in 0..12 {
        assert!(undo.undo(&mut session), "Undo {i} should succeed");
    }

    // Verify we are back to original.
    assert!(!undo.can_undo(), "All operations undone");
    assert_eq!(undo.redo_count(), 12);
    assert_eq!(
        session.track(track_id).unwrap().regions.len(),
        original_region_count,
        "Should have no regions after full undo"
    );
    assert!(
        (session.track(track_id).unwrap().gain - original_gain).abs() < 1e-6,
        "Gain should be back to original"
    );
    assert!(
        (session.track(track_id).unwrap().pan - original_pan).abs() < 1e-6,
        "Pan should be back to original"
    );
    assert!(
        !session.track(track_id).unwrap().muted,
        "Should not be muted"
    );
    assert!(
        !session.track(track_id).unwrap().solo,
        "Should not be soloed"
    );
    assert_eq!(
        session.track(track2_id).unwrap().regions.len(),
        0,
        "Track 2 should have no regions"
    );

    // Redo ALL 12 operations.
    for i in 0..12 {
        assert!(undo.redo(&mut session), "Redo {i} should succeed");
    }

    // Verify we are back to the edited state.
    assert!(!undo.can_redo(), "All operations redone");
    assert_eq!(undo.undo_count(), 12);
    assert_eq!(
        session.track(track_id).unwrap().regions.len(),
        edited_track1_regions,
        "Should have edited region count after full redo"
    );
    assert!(
        (session.track(track_id).unwrap().gain - edited_gain).abs() < 1e-6,
        "Gain should match edited state"
    );
    assert!(session.track(track_id).unwrap().muted, "Should be muted");
    assert!(session.track(track_id).unwrap().solo, "Should be soloed");
}

// ===========================================================================
// 7. Audio format roundtrip
// ===========================================================================

#[test]
fn audio_format_roundtrip_wav_float32() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("roundtrip_f32.wav");

    let sample_rate = 48000u32;
    let frames = 4800;

    // Generate a test signal: two sine waves mixed.
    let mut data = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let left = (2.0 * PI * 440.0 * t).sin() * 0.7;
        let right = (2.0 * PI * 880.0 * t).sin() * 0.4;
        data.push(left);
        data.push(right);
    }

    let original = AudioBuffer::from_interleaved(data, 2);
    let format = AudioFormat::new(sample_rate, 2, 0);
    write_wav_file(&path, &original, &format).unwrap();

    let (loaded, loaded_format) = read_audio_file(&path).unwrap();
    assert_eq!(loaded_format.sample_rate, sample_rate);
    assert_eq!(loaded_format.channels, 2);
    assert_eq!(loaded.frames(), original.frames());

    for i in 0..original.sample_count() {
        let diff = (original.as_interleaved()[i] - loaded.as_interleaved()[i]).abs();
        assert!(
            diff < 1e-6,
            "Float32 roundtrip: sample {i} differs by {diff}"
        );
    }
}

#[test]
fn audio_format_roundtrip_wav_int16() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("roundtrip_i16.wav");

    let frames = 2400;
    let mut data = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / 48000.0;
        let left = (2.0 * PI * 440.0 * t).sin() * 0.5;
        let right = (2.0 * PI * 660.0 * t).sin() * 0.3;
        data.push(left);
        data.push(right);
    }

    let original = AudioBuffer::from_interleaved(data, 2);
    let config = ExportConfig {
        format: ExportFormat::Wav,
        bit_depth: BitDepth::Int16,
        sample_rate: 48000,
        channels: 2,
    };

    write_audio_file(&path, &original, &config).unwrap();
    let (loaded, _) = read_audio_file(&path).unwrap();

    assert_eq!(loaded.frames(), original.frames());

    // 16-bit quantization tolerance.
    let tolerance = 1.0 / 32768.0 + 1e-4;
    for i in 0..original.sample_count() {
        let diff = (original.as_interleaved()[i] - loaded.as_interleaved()[i]).abs();
        assert!(
            diff < tolerance,
            "Int16 roundtrip: sample {i} differs by {diff} (tolerance {tolerance})"
        );
    }
}

#[test]
fn audio_format_roundtrip_wav_int24() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("roundtrip_i24.wav");

    let frames = 2400;
    let mut data = Vec::with_capacity(frames);
    for i in 0..frames {
        let t = i as f32 / 48000.0;
        data.push((2.0 * PI * 1000.0 * t).sin() * 0.8);
    }

    let original = AudioBuffer::from_interleaved(data, 1);
    let config = ExportConfig {
        format: ExportFormat::Wav,
        bit_depth: BitDepth::Int24,
        sample_rate: 48000,
        channels: 1,
    };

    write_audio_file(&path, &original, &config).unwrap();
    let (loaded, _) = read_audio_file(&path).unwrap();

    let tolerance = 1.0 / 8_388_607.0 + 1e-6;
    for i in 0..original.sample_count() {
        let diff = (original.as_interleaved()[i] - loaded.as_interleaved()[i]).abs();
        assert!(
            diff < tolerance,
            "Int24 roundtrip: sample {i} differs by {diff}"
        );
    }
}

// ===========================================================================
// 8. Preset roundtrip for all instruments
// ===========================================================================

#[test]
fn preset_roundtrip_subtractive_synth() {
    let mut synth = SubtractiveSynth::new(48000.0);

    // Set non-default params: find Volume and set it.
    if let Some(vol) = synth.params_mut().iter_mut().find(|p| p.name == "Volume") {
        vol.set(0.42);
    }
    if let Some(atk) = synth.params_mut().iter_mut().find(|p| p.name == "Attack") {
        atk.set(0.15);
    }
    if let Some(rel) = synth.params_mut().iter_mut().find(|p| p.name == "Release") {
        rel.set(0.9);
    }

    let preset = InstrumentPreset::from_instrument(&synth, "Custom Synth");

    // Save and load via JSON (in-memory).
    let json = serde_json::to_string(&preset).unwrap();
    let loaded_preset: InstrumentPreset = serde_json::from_str(&json).unwrap();

    // Apply to a fresh synth.
    let mut synth2 = SubtractiveSynth::new(48000.0);
    loaded_preset.apply_to(&mut synth2);

    // Verify all params match.
    for (orig, restored) in synth.params().iter().zip(synth2.params().iter()) {
        assert_eq!(orig.name, restored.name);
        assert!(
            (orig.value - restored.value).abs() < f32::EPSILON,
            "Param '{}' mismatch: {} vs {}",
            orig.name,
            orig.value,
            restored.value
        );
    }
}

#[test]
fn preset_roundtrip_drum_machine() {
    let mut dm = DrumMachine::new(48000.0);

    // Set some non-default params.
    if let Some(vol) = dm.params_mut().iter_mut().find(|p| p.name == "Volume") {
        vol.set(0.6);
    }

    let preset = InstrumentPreset::from_instrument(&dm, "Custom Kit");
    let json = serde_json::to_string(&preset).unwrap();
    let loaded_preset: InstrumentPreset = serde_json::from_str(&json).unwrap();

    let mut dm2 = DrumMachine::new(48000.0);
    loaded_preset.apply_to(&mut dm2);

    for (orig, restored) in dm.params().iter().zip(dm2.params().iter()) {
        assert_eq!(orig.name, restored.name);
        assert!(
            (orig.value - restored.value).abs() < f32::EPSILON,
            "Param '{}' mismatch: {} vs {}",
            orig.name,
            orig.value,
            restored.value
        );
    }
}

#[test]
fn preset_roundtrip_sampler() {
    let mut sampler = Sampler::new(48000.0);

    if let Some(vol) = sampler.params_mut().iter_mut().find(|p| p.name == "Volume") {
        vol.set(0.35);
    }

    let preset = InstrumentPreset::from_instrument(&sampler, "Custom Sampler");
    let json = serde_json::to_string(&preset).unwrap();
    let loaded_preset: InstrumentPreset = serde_json::from_str(&json).unwrap();

    let mut sampler2 = Sampler::new(48000.0);
    loaded_preset.apply_to(&mut sampler2);

    for (orig, restored) in sampler.params().iter().zip(sampler2.params().iter()) {
        assert_eq!(orig.name, restored.name);
        assert!(
            (orig.value - restored.value).abs() < f32::EPSILON,
            "Param '{}' mismatch: {} vs {}",
            orig.name,
            orig.value,
            restored.value
        );
    }
}

#[test]
fn preset_file_roundtrip_all_instruments() {
    let dir = tempfile::tempdir().unwrap();

    let instruments: Vec<(&str, Box<dyn InstrumentNode>)> = vec![
        ("synth", Box::new(SubtractiveSynth::new(48000.0))),
        ("drum_machine", Box::new(DrumMachine::new(48000.0))),
        ("sampler", Box::new(Sampler::new(48000.0))),
    ];

    for (name, inst) in &instruments {
        let preset = InstrumentPreset::from_instrument(inst.as_ref(), name);
        let path = dir.path().join(format!("{name}.json"));
        preset.save(&path).unwrap();
        let loaded = InstrumentPreset::load(&path).unwrap();

        assert_eq!(loaded.name, *name);
        assert_eq!(loaded.params.len(), preset.params.len());
        for (a, b) in loaded.params.iter().zip(preset.params.iter()) {
            assert_eq!(a.name, b.name);
            assert!(
                (a.value - b.value).abs() < f32::EPSILON,
                "Instrument '{}' param '{}' mismatch",
                name,
                a.name
            );
        }
    }
}

// ===========================================================================
// 9. MIDI routing
// ===========================================================================

#[test]
fn midi_routing_velocity_curve_channel_filter_note_range() {
    // Create a set of MIDI events.
    let mut clip = MidiClip::new("Test Clip", 0, 48000);
    clip.add_note(0, 1000, 60, 100, 0); // C4, vel 100, ch 0
    clip.add_note(0, 1000, 48, 80, 1); // C3, vel 80, ch 1
    clip.add_note(0, 1000, 72, 120, 0); // C5, vel 120, ch 0
    clip.add_note(0, 1000, 84, 60, 0); // C6, vel 60, ch 0 (above range)
    clip.add_note(0, 1000, 30, 90, 0); // below range

    // Route: channel 0 only, note range 36-72, soft velocity curve.
    let mut route = MidiRoute::new(uuid::Uuid::new_v4());
    route.channel_filter = Some(0);
    route.note_range = (36, 72);
    route.velocity_curve = VelocityCurve::Soft;

    // Apply route to each event.
    let mut passed: Vec<NoteEvent> = Vec::new();
    for note in &clip.notes {
        if let Some(transformed) = route.filter_event(note) {
            passed.push(transformed);
        }
    }

    // Should pass: C4 (60, ch0), C5 (72, ch0).
    // Rejected: C3 (48, ch1 - wrong channel), C6 (84, ch0 - above range), note 30 (below range).
    assert_eq!(
        passed.len(),
        2,
        "Only 2 events should pass: got {}",
        passed.len()
    );

    // Sort by note for deterministic ordering (clip may order same-position notes arbitrarily).
    passed.sort_by_key(|n| n.note);
    assert_eq!(passed[0].note, 60);
    assert_eq!(passed[1].note, 72);

    // Verify soft velocity curve was applied (soft boosts low-mid velocities).
    let soft = VelocityCurve::Soft;
    assert_eq!(passed[0].velocity, soft.apply(100));
    assert_eq!(passed[1].velocity, soft.apply(120));

    // Soft curve: mid-range velocity should be boosted.
    let mid_vel = soft.apply(50);
    assert!(
        mid_vel > 50,
        "Soft curve should boost mid velocity: 50 -> {mid_vel}"
    );
}

#[test]
fn midi_routing_fixed_velocity() {
    let mut route = MidiRoute::new(uuid::Uuid::new_v4());
    route.velocity_curve = VelocityCurve::Fixed(80);

    let event = NoteEvent {
        position: 0,
        duration: 100,
        note: 60,
        velocity: 127,
        channel: 0,
    };

    let result = route.filter_event(&event).unwrap();
    assert_eq!(result.velocity, 80, "Fixed velocity should override input");
}

#[test]
fn midi_routing_hard_velocity_curve() {
    let mut route = MidiRoute::new(uuid::Uuid::new_v4());
    route.velocity_curve = VelocityCurve::Hard;

    let event = NoteEvent {
        position: 0,
        duration: 100,
        note: 60,
        velocity: 90,
        channel: 0,
    };

    let result = route.filter_event(&event).unwrap();
    // Hard curve squares the normalized value, so mid-range velocities are reduced.
    assert!(
        result.velocity < 90,
        "Hard curve should reduce mid-range velocity: 90 -> {}",
        result.velocity
    );
}

// ===========================================================================
// 10. Sampler zone mapping
// ===========================================================================

#[test]
fn sampler_zone_mapping_correct_zone_selection() {
    let sample_rate = 48000.0f32;
    let buffer_size = 2048u32;

    let mut sampler = Sampler::new(sample_rate);

    // Create three zones:
    // Zone 1: C2-B3 (36-59), root = 48 (C3)
    let mut zone_low = SampleZone::new(
        "Bass",
        48,
        generate_sine(100.0, sample_rate, 4800, 0.5),
        sample_rate as u32,
    );
    zone_low.key_low = 36;
    zone_low.key_high = 59;

    // Zone 2: C4-B5 (60-83), root = 72 (C5)
    let mut zone_mid = SampleZone::new(
        "Mid",
        72,
        generate_sine(500.0, sample_rate, 4800, 0.5),
        sample_rate as u32,
    );
    zone_mid.key_low = 60;
    zone_mid.key_high = 83;

    // Zone 3: C6-C8 (84-108), root = 96 (C7)
    let mut zone_high = SampleZone::new(
        "High",
        96,
        generate_sine(2000.0, sample_rate, 4800, 0.5),
        sample_rate as u32,
    );
    zone_high.key_low = 84;
    zone_high.key_high = 108;

    sampler.add_zone(zone_low);
    sampler.add_zone(zone_mid);
    sampler.add_zone(zone_high);

    // Play note in the low zone.
    sampler.note_on(48, 100, 0); // C3 -> should use Bass zone
    let mut output_low = AudioBuffer::new(2, buffer_size);
    sampler.process(&[], &[], &mut output_low);
    let has_low = has_audio(&output_low, 0.001);

    sampler.reset();

    // Play note in the mid zone.
    sampler.note_on(72, 100, 0); // C5 -> should use Mid zone
    let mut output_mid = AudioBuffer::new(2, buffer_size);
    sampler.process(&[], &[], &mut output_mid);
    let has_mid = has_audio(&output_mid, 0.001);

    sampler.reset();

    // Play note in the high zone.
    sampler.note_on(96, 100, 0); // C7 -> should use High zone
    let mut output_high = AudioBuffer::new(2, buffer_size);
    sampler.process(&[], &[], &mut output_high);
    let has_high = has_audio(&output_high, 0.001);

    assert!(has_low, "Low zone (note 48) should produce audio");
    assert!(has_mid, "Mid zone (note 72) should produce audio");
    assert!(has_high, "High zone (note 96) should produce audio");

    // Play a note outside all zones.
    sampler.reset();
    sampler.note_on(20, 100, 0); // Well below all zones
    let mut output_none = AudioBuffer::new(2, buffer_size);
    sampler.process(&[], &[], &mut output_none);
    assert!(
        !has_audio(&output_none, 0.001),
        "Note outside all zones should produce no audio"
    );
}

#[test]
fn sampler_zone_pitch_mapping() {
    let sample_rate = 48000.0f32;
    let buffer_size = 4096u32;

    let mut sampler = Sampler::new(sample_rate);

    // Single zone with root at C4 (60), covering the full keyboard.
    let zone = SampleZone::new(
        "Piano",
        60,
        generate_sine(261.63, sample_rate, 48000, 0.5), // C4 frequency
        sample_rate as u32,
    );
    sampler.add_zone(zone);

    // Play root note (C4 = 60): should play at original pitch.
    sampler.note_on(60, 100, 0);
    let mut output_root = AudioBuffer::new(2, buffer_size);
    sampler.process(&[], &[], &mut output_root);

    sampler.reset();

    // Play one octave up (C5 = 72): should play faster (higher pitch).
    sampler.note_on(72, 100, 0);
    let mut output_octave_up = AudioBuffer::new(2, buffer_size);
    sampler.process(&[], &[], &mut output_octave_up);

    // Both should produce audio.
    assert!(
        has_audio(&output_root, 0.001),
        "Root note should produce audio"
    );
    assert!(
        has_audio(&output_octave_up, 0.001),
        "Octave-up note should produce audio"
    );

    // Count zero crossings to verify pitch difference.
    // Octave up should have roughly 2x the zero crossings.
    fn count_zero_crossings(buf: &AudioBuffer, channel: u16, frames: u32) -> usize {
        let mut count = 0;
        for i in 1..frames {
            let prev = buf.get(i - 1, channel);
            let curr = buf.get(i, channel);
            if (prev >= 0.0 && curr < 0.0) || (prev < 0.0 && curr >= 0.0) {
                count += 1;
            }
        }
        count
    }

    let crossings_root = count_zero_crossings(&output_root, 0, buffer_size);
    let crossings_up = count_zero_crossings(&output_octave_up, 0, buffer_size);

    // The octave-up should have roughly 2x crossings (within tolerance).
    if crossings_root > 5 {
        let ratio = crossings_up as f32 / crossings_root as f32;
        assert!(
            ratio > 1.5 && ratio < 2.5,
            "Octave up should have ~2x zero crossings: root={crossings_root}, up={crossings_up}, ratio={ratio}"
        );
    }
}

// ===========================================================================
// Additional cross-crate tests
// ===========================================================================

#[test]
fn session_with_bus_sends_renders_correctly() {
    let sample_rate = 48000u32;
    let buffer_size = 512u32;

    let mut session = Session::new("Bus Send Test", sample_rate, buffer_size);
    let audio_id = session.add_audio_track("Source");
    let bus_id = session.add_bus_track("FX Bus");

    // Load audio into the pool.
    let sine: Vec<f32> = generate_sine(440.0, sample_rate as f32, buffer_size as usize, 0.5)
        .iter()
        .flat_map(|&s| vec![s, s])
        .collect();
    session
        .audio_pool
        .insert("source.wav".into(), AudioBuffer::from_interleaved(sine, 2));

    // Add region.
    session.track_mut(audio_id).unwrap().add_region(Region::new(
        "source.wav".into(),
        0,
        0,
        buffer_size as u64,
    ));

    // Add a post-fader send to the bus.
    session.add_send(audio_id, bus_id, 0.5, SendPosition::PostFader);

    // Render.
    let mut output = AudioBuffer::new(2, buffer_size);
    let timeline = session.timeline.as_mut().unwrap();
    timeline.render(
        &session.tracks,
        &session.transport,
        &session.audio_pool,
        &mut output,
    );

    // Output should be louder than the direct signal alone (bus contribution).
    let direct_max = 0.5f32; // original sine amplitude
    let output_max = output
        .as_interleaved()
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);

    assert!(
        output_max > direct_max,
        "Bus send should add to the output: direct_max={direct_max}, output_max={output_max}"
    );
}

#[test]
fn automation_affects_timeline_render() {
    let sample_rate = 48000u32;
    let buffer_size = 512u32;

    let mut session = Session::new("Automation Test", sample_rate, buffer_size);
    let track_id = session.add_audio_track("Automated");

    // Load a constant signal.
    let constant: Vec<f32> = vec![0.5; buffer_size as usize * 2]; // stereo
    session.audio_pool.insert(
        "constant.wav".into(),
        AudioBuffer::from_interleaved(constant, 2),
    );

    session.track_mut(track_id).unwrap().add_region(Region::new(
        "constant.wav".into(),
        0,
        0,
        buffer_size as u64,
    ));

    // Render WITHOUT automation first (track gain defaults to 1.0).
    let mut output_no_auto = AudioBuffer::new(2, buffer_size);
    {
        let timeline = session.timeline.as_mut().unwrap();
        timeline.render(
            &session.tracks,
            &session.transport,
            &session.audio_pool,
            &mut output_no_auto,
        );
    }
    let sample_no_auto = output_no_auto.get(100, 0);

    // Add gain automation: constant value of 0.25 at position 0.
    // The timeline reads automation at the transport position (0), so
    // this should override the track gain from 1.0 to 0.25 for the whole buffer.
    let mut auto_lane = AutomationLane::new(AutomationTarget::TrackGain);
    auto_lane.add_point(AutomationPoint {
        position: 0,
        value: 0.25,
        curve: CurveType::Linear,
    });
    session
        .track_mut(track_id)
        .unwrap()
        .automation
        .push(auto_lane);

    // Render WITH automation.
    let mut output_auto = AudioBuffer::new(2, buffer_size);
    {
        let timeline = session.timeline.as_mut().unwrap();
        timeline.render(
            &session.tracks,
            &session.transport,
            &session.audio_pool,
            &mut output_auto,
        );
    }
    let sample_auto = output_auto.get(100, 0);

    // With automation setting gain to 0.25, the output should be ~1/4 of the
    // no-automation output (which uses the default gain of 1.0).
    assert!(
        sample_no_auto.abs() > 0.01,
        "Without automation, output should be non-silent: {sample_no_auto}"
    );
    assert!(
        sample_auto.abs() > 0.01,
        "With automation at 0.25 gain, output should be non-silent: {sample_auto}"
    );
    let ratio = sample_auto / sample_no_auto;
    assert!(
        (ratio - 0.25).abs() < 0.05,
        "Automation should set gain to ~0.25: ratio={ratio}"
    );
}
