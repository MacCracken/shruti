use criterion::{Criterion, criterion_group, criterion_main};
use shruti_dsp::AudioBuffer;
use shruti_instruments::{
    DrumMachine, InstrumentNode, SampleZone, Sampler, SamplerParam, SubtractiveSynth, SynthParam,
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
            |mut buf| synth.process(&[], &[], &mut buf),
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
            |mut buf| synth.process(&[], &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Synth: Unison ──────────────────────────────────────────────

fn synth_unison_4_render_256(c: &mut Criterion) {
    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    synth.set_param(SynthParam::UnisonVoices, 4.0);
    synth.set_param(SynthParam::UnisonDetune, 20.0);
    synth.set_param(SynthParam::UnisonSpread, 0.5);
    synth.note_on(60, 100, 0);

    c.bench_function("synth_unison_4_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| synth.process(&[], &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn synth_unison_8_render_256(c: &mut Criterion) {
    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    synth.set_param(SynthParam::UnisonVoices, 8.0);
    synth.set_param(SynthParam::UnisonDetune, 50.0);
    synth.set_param(SynthParam::UnisonSpread, 1.0);
    synth.note_on(60, 100, 0);

    c.bench_function("synth_unison_8_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| synth.process(&[], &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Synth: Sub-oscillator ──────────────────────────────────────

fn synth_sub_osc_render_256(c: &mut Criterion) {
    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    synth.set_param(SynthParam::SubOscEnable, 1.0);
    synth.set_param(SynthParam::SubOscLevel, 0.8);
    synth.note_on(60, 100, 0);

    c.bench_function("synth_sub_osc_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| synth.process(&[], &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Synth: Full stack (unison + sub + osc2 + filter mod) ───────

fn synth_full_stack_render_256(c: &mut Criterion) {
    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    synth.set_param(SynthParam::UnisonVoices, 4.0);
    synth.set_param(SynthParam::UnisonDetune, 15.0);
    synth.set_param(SynthParam::SubOscEnable, 1.0);
    synth.set_param(SynthParam::Osc2Enable, 1.0);
    synth.set_param(SynthParam::FilterCutoff, 2000.0);
    synth.set_param(SynthParam::FilterResonance, 0.5);
    synth.set_param(SynthParam::FilterEnvDepth, 0.7);
    synth.set_param(SynthParam::Lfo1Depth, 0.3);
    synth.set_param(SynthParam::Lfo1Target, 1.0);
    synth.note_on(60, 100, 0);
    synth.note_on(64, 80, 0);
    synth.note_on(67, 90, 0);

    c.bench_function("synth_full_stack_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| synth.process(&[], &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Synth: Polyphony scaling ───────────────────────────────────

fn synth_16_voices_render_256(c: &mut Criterion) {
    let mut synth = SubtractiveSynth::new(SAMPLE_RATE);
    for note in 40..56 {
        synth.note_on(note, 100, 0);
    }

    c.bench_function("synth_16_voices_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| synth.process(&[], &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── DrumMachine ─────────────────────────────────────────────────

fn drum_machine_render_256(c: &mut Criterion) {
    let mut dm = DrumMachine::new(SAMPLE_RATE);
    let sample_len = 4800;
    let samples: Vec<f32> = (0..sample_len)
        .map(|i| (2.0 * std::f32::consts::PI * 200.0 * i as f32 / SAMPLE_RATE).sin() * 0.8)
        .collect();
    dm.pads[0].samples = samples;
    dm.pads[0].sample_rate = SAMPLE_RATE as u32;
    let events = vec![note_event(36, 100)];

    c.bench_function("drum_machine_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| dm.process(&events, &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Sampler ─────────────────────────────────────────────────────

fn sampler_render_256(c: &mut Criterion) {
    let mut sampler = Sampler::new(SAMPLE_RATE);
    let samples: Vec<f32> = (0..48000)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / SAMPLE_RATE).sin() * 0.5)
        .collect();
    let zone = SampleZone::new("Sine C4", 60, samples, SAMPLE_RATE as u32);
    sampler.add_zone(zone);
    sampler.note_on(60, 100, 0);

    c.bench_function("sampler_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| sampler.process(&[], &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Sampler: Time-stretching ───────────────────────────────────

fn sampler_time_stretch_render_256(c: &mut Criterion) {
    let mut sampler = Sampler::new(SAMPLE_RATE);
    let samples: Vec<f32> = (0..48000)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / SAMPLE_RATE).sin() * 0.5)
        .collect();
    let zone = SampleZone::new("Sine C4", 60, samples, SAMPLE_RATE as u32);
    sampler.add_zone(zone);
    sampler.set_param(SamplerParam::TimeStretch, 0.5);
    sampler.note_on(60, 100, 0);

    c.bench_function("sampler_time_stretch_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| sampler.process(&[], &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Groups ──────────────────────────────────────────────────────

criterion_group!(
    synth_benches,
    synth_render_256,
    synth_render_1024,
    synth_unison_4_render_256,
    synth_unison_8_render_256,
    synth_sub_osc_render_256,
    synth_full_stack_render_256,
    synth_16_voices_render_256,
);

criterion_group!(drum_benches, drum_machine_render_256,);

criterion_group!(
    sampler_benches,
    sampler_render_256,
    sampler_time_stretch_render_256,
);

criterion_main!(synth_benches, drum_benches, sampler_benches);
