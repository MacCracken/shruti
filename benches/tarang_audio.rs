//! Benchmarks for tarang audio backend integration.
//!
//! Requires the `tarang` feature: `cargo bench --features tarang --bench tarang_audio`

use criterion::{Criterion, criterion_group, criterion_main};

use shruti_dsp::buffer::AudioBuffer;
use shruti_dsp::format::AudioFormat;
use shruti_dsp::io::mix::{ChannelLayout, mix_channels};
use shruti_dsp::io::resample::{resample, resample_sinc};
use shruti_dsp::io::writer::{BitDepth, ExportConfig, ExportFormat, write_audio_file};
use shruti_dsp::io::{read_audio_file, write_wav_file};

/// Generate a stereo sine wave at the given sample rate and duration.
fn sine_stereo(sample_rate: u32, duration_secs: f32) -> AudioBuffer {
    let frames = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let s = (t * 440.0 * std::f32::consts::TAU).sin() * 0.5;
        samples.push(s); // L
        samples.push(s); // R
    }
    AudioBuffer::from_interleaved(samples, 2)
}

/// Generate a mono sine wave at the given sample rate and duration.
fn sine_mono(sample_rate: u32, duration_secs: f32) -> AudioBuffer {
    let frames = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(frames);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        samples.push((t * 440.0 * std::f32::consts::TAU).sin() * 0.5);
    }
    AudioBuffer::from_interleaved(samples, 1)
}

// ── File I/O benchmarks ──────────────────────────────────────────────────

fn bench_io(c: &mut Criterion) {
    let mut group = c.benchmark_group("tarang_io");

    let buf = sine_stereo(48000, 1.0);
    let fmt = AudioFormat::new(48000, 2, 0);
    let tmp_dir = std::env::temp_dir().join("shruti_bench_tarang");
    std::fs::create_dir_all(&tmp_dir).unwrap();

    // Write a WAV file for read benchmarks
    let wav_path = tmp_dir.join("bench.wav");
    write_wav_file(&wav_path, &buf, &fmt).unwrap();

    group.bench_function("wav_read_1s_stereo", |b| {
        b.iter(|| read_audio_file(&wav_path).unwrap());
    });

    group.bench_function("wav_write_1s_stereo", |b| {
        let out = tmp_dir.join("bench_write.wav");
        b.iter(|| write_wav_file(&out, &buf, &fmt).unwrap());
    });

    group.bench_function("flac_export_1s_stereo_16bit", |b| {
        let out = tmp_dir.join("bench.flac");
        let cfg = ExportConfig {
            format: ExportFormat::Flac,
            bit_depth: BitDepth::Int16,
            sample_rate: 48000,
            channels: 2,
        };
        b.iter(|| write_audio_file(&out, &buf, &cfg).unwrap());
    });

    group.bench_function("flac_export_1s_stereo_24bit", |b| {
        let out = tmp_dir.join("bench_24.flac");
        let cfg = ExportConfig {
            format: ExportFormat::Flac,
            bit_depth: BitDepth::Int24,
            sample_rate: 48000,
            channels: 2,
        };
        b.iter(|| write_audio_file(&out, &buf, &cfg).unwrap());
    });

    group.finish();
    let _ = std::fs::remove_dir_all(&tmp_dir);
}

// ── Resampling benchmarks (0.25s buffers — processing-bound) ─────────────
//
// NOTE: tarang 0.19.3 has an off-by-one in the linear resampler that panics
// on stereo buffers processed through shruti's shruti_to_tarang() conversion.
// Skipping linear stereo and sinc stereo until tarang 0.20.3 fixes it.
// Mono benchmarks work fine.

fn bench_resample(c: &mut Criterion) {
    let mut group = c.benchmark_group("tarang_resample");
    group.sample_size(20);

    let mono_44 = sine_mono(44100, 0.25);
    let mono_48 = sine_mono(48000, 0.25);

    group.bench_function("linear_44100_to_48000_mono", |b| {
        b.iter(|| resample(&mono_44, 44100, 48000).unwrap());
    });

    group.bench_function("linear_48000_to_16000_mono", |b| {
        b.iter(|| resample(&mono_48, 48000, 16000).unwrap());
    });

    group.bench_function("sinc_44100_to_48000_mono_w16", |b| {
        b.iter(|| resample_sinc(&mono_44, 44100, 48000, 16).unwrap());
    });

    group.bench_function("sinc_44100_to_48000_mono_w64", |b| {
        b.iter(|| resample_sinc(&mono_44, 44100, 48000, 64).unwrap());
    });

    group.finish();
}

// ── Channel mixing benchmarks ─────────────────────────────────────────────
//
// NOTE: tarang 0.19.3 has a stereo buffer size mismatch in mix_channels
// (expects num_samples = frames * channels * something). Skipped until 0.20.3.
// The mono→stereo path works, but stereo→mono triggers the same off-by-one.

fn bench_mix(c: &mut Criterion) {
    let mut group = c.benchmark_group("tarang_mix");
    group.sample_size(20);

    let mono = sine_mono(48000, 0.25);

    group.bench_function("mono_to_stereo", |b| {
        b.iter(|| mix_channels(&mono, 48000, ChannelLayout::Stereo).unwrap());
    });

    group.finish();
}

criterion_group!(io_benches, bench_io);
criterion_group!(resample_benches, bench_resample);
criterion_group!(mix_benches, bench_mix);
criterion_main!(io_benches, resample_benches, mix_benches);
