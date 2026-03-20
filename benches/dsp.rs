use criterion::{Criterion, criterion_group, criterion_main};
use shruti_dsp::AudioBuffer;
use shruti_dsp::effects::eq::{EqBand, FilterType};
use shruti_dsp::effects::{Compressor, Delay, Limiter, ParametricEq, Reverb};

const SAMPLE_RATE: f32 = 48000.0;
const CHANNELS: u16 = 2;
const FRAMES_1S: u32 = 48000;

/// Generate a 1-second stereo buffer filled with a 440 Hz sine wave.
fn sine_buffer_1s() -> AudioBuffer {
    let total = CHANNELS as usize * FRAMES_1S as usize;
    let mut data = Vec::with_capacity(total);
    for i in 0..FRAMES_1S as usize {
        let sample = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / SAMPLE_RATE).sin() * 0.5;
        for _ in 0..CHANNELS {
            data.push(sample);
        }
    }
    AudioBuffer::from_interleaved(data, CHANNELS)
}

// ── EQ ──────────────────────────────────────────────────────────────

fn eq_process(c: &mut Criterion) {
    let mut eq = ParametricEq::new(SAMPLE_RATE);
    eq.add_band(EqBand::new(FilterType::LowShelf, 200.0, 3.0, 0.707));
    eq.add_band(EqBand::new(FilterType::Peak, 1000.0, -2.0, 1.0));
    eq.add_band(EqBand::new(FilterType::HighShelf, 8000.0, 2.0, 0.707));

    c.bench_function("eq_process", |b| {
        b.iter_batched(
            || sine_buffer_1s(),
            |mut buf| {
                eq.reset();
                eq.process(&mut buf);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Compressor ──────────────────────────────────────────────────────

fn compressor_process(c: &mut Criterion) {
    let mut comp = Compressor::new(SAMPLE_RATE);
    comp.threshold_db = -18.0;
    comp.ratio = 4.0;
    comp.attack = 0.005;
    comp.release = 0.05;
    comp.makeup_db = 3.0;
    comp.knee_db = 6.0;

    c.bench_function("compressor_process", |b| {
        b.iter_batched(
            || sine_buffer_1s(),
            |mut buf| {
                comp.reset();
                comp.process(&mut buf);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Reverb ──────────────────────────────────────────────────────────

fn reverb_process(c: &mut Criterion) {
    let mut reverb = Reverb::new(SAMPLE_RATE);
    reverb.mix = 0.3;
    reverb.room_size = 0.6;
    reverb.damping = 0.5;
    reverb.update_parameters();

    c.bench_function("reverb_process", |b| {
        b.iter_batched(
            || sine_buffer_1s(),
            |mut buf| {
                reverb.reset();
                reverb.process(&mut buf);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Delay ───────────────────────────────────────────────────────────

fn delay_process(c: &mut Criterion) {
    let mut delay = Delay::new(SAMPLE_RATE);
    delay.time = 0.5;
    delay.feedback = 0.5;
    delay.mix = 0.4;

    c.bench_function("delay_process", |b| {
        b.iter_batched(
            || sine_buffer_1s(),
            |mut buf| {
                delay.reset();
                delay.process(&mut buf);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Limiter ─────────────────────────────────────────────────────────

fn limiter_process(c: &mut Criterion) {
    let mut limiter = Limiter::new(SAMPLE_RATE);
    limiter.ceiling_db = -1.0;
    limiter.release = 0.01;

    c.bench_function("limiter_process", |b| {
        b.iter_batched(
            || sine_buffer_1s(),
            |mut buf| {
                limiter.reset();
                limiter.process(&mut buf);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── AudioBuffer operations ──────────────────────────────────────────

fn buffer_from_interleaved(c: &mut Criterion) {
    let total = CHANNELS as usize * FRAMES_1S as usize;
    let data: Vec<f32> = (0..total).map(|i| (i as f32 * 0.001).sin()).collect();

    c.bench_function("buffer_from_interleaved", |b| {
        b.iter_batched(
            || data.clone(),
            |d| AudioBuffer::from_interleaved(d, CHANNELS),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn buffer_mix_from(c: &mut Criterion) {
    let buf_a = sine_buffer_1s();
    let buf_b = sine_buffer_1s();

    c.bench_function("buffer_mix_from", |b| {
        b.iter_batched(
            || (buf_a.clone(), buf_b.clone()),
            |(mut a, ref b_ref)| a.mix_from(b_ref),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn buffer_apply_gain(c: &mut Criterion) {
    c.bench_function("buffer_apply_gain", |b| {
        b.iter_batched(
            || sine_buffer_1s(),
            |mut buf| buf.apply_gain(0.8),
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Groups ──────────────────────────────────────────────────────────

criterion_group!(
    effects,
    eq_process,
    compressor_process,
    reverb_process,
    delay_process,
    limiter_process,
);

criterion_group!(
    buffer_ops,
    buffer_from_interleaved,
    buffer_mix_from,
    buffer_apply_gain,
);

criterion_main!(effects, buffer_ops);
