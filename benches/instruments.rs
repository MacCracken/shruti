use criterion::{Criterion, criterion_group, criterion_main};
use shruti_dsp::AudioBuffer;
use shruti_instruments::{
    DrumMachine, InstrumentNode, Sampler, SampleZone, SubtractiveSynth,
};
use shruti_session::midi::NoteEvent;
use shruti_session::types::FramePos;

const SAMPLE_RATE: f32 = 48000.0;

/// Create a NoteEvent for benchmarking (note-on at position 0).
fn note_event(note: u8, velocity: u8) -> NoteEvent {
    NoteEvent {
        position: FramePos(0),
        duration: FramePos(48000),
        note,
        velocity,
        channel: 0,
    }
}

// ── SubtractiveSynth ────────────────────────────────────────────────

fn synth_render_256(c: &mut Criterion) {
    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    synth.note_on(60, 100, 0);

    c.bench_function("synth_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| {
                synth.process(&[], &[], &mut buf);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn synth_render_1024(c: &mut Criterion) {
    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    synth.note_on(60, 100, 0);

    c.bench_function("synth_render_1024", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 1024),
            |mut buf| {
                synth.process(&[], &[], &mut buf);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── DrumMachine ─────────────────────────────────────────────────────

fn drum_machine_render_256(c: &mut Criterion) {
    let mut dm = DrumMachine::new(SAMPLE_RATE);

    // Load a short sine sample into pad 0 (MIDI note 36)
    let sample_len = 4800; // 100ms at 48kHz
    let samples: Vec<f32> = (0..sample_len)
        .map(|i| (2.0 * std::f32::consts::PI * 200.0 * i as f32 / SAMPLE_RATE).sin() * 0.8)
        .collect();
    dm.pads[0].samples = samples;
    dm.pads[0].sample_rate = SAMPLE_RATE as u32;

    // Trigger pad 0
    let events = vec![note_event(36, 100)];

    c.bench_function("drum_machine_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| {
                dm.process(&events, &[], &mut buf);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Sampler ─────────────────────────────────────────────────────────

fn sampler_render_256(c: &mut Criterion) {
    let mut sampler = Sampler::new(SAMPLE_RATE);

    // Create a zone with a short sine sample mapped to note 60
    let sample_len = 48000; // 1s at 48kHz
    let samples: Vec<f32> = (0..sample_len)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / SAMPLE_RATE).sin() * 0.5)
        .collect();
    let zone = SampleZone::new("Sine C4", 60, samples, SAMPLE_RATE as u32);
    sampler.add_zone(zone);

    // Trigger note 60
    sampler.note_on(60, 100, 0);

    c.bench_function("sampler_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| {
                sampler.process(&[], &[], &mut buf);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Groups ──────────────────────────────────────────────────────────

criterion_group!(
    synth_benches,
    synth_render_256,
    synth_render_1024,
);

criterion_group!(
    drum_benches,
    drum_machine_render_256,
);

criterion_group!(
    sampler_benches,
    sampler_render_256,
);

criterion_main!(synth_benches, drum_benches, sampler_benches);
