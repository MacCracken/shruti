use criterion::{Criterion, criterion_group, criterion_main};
use shruti_dsp::AudioBuffer;
use shruti_session::audio_pool::AudioPool;
use shruti_session::region::Region;
use shruti_session::timeline::Timeline;
use shruti_session::track::Track;
use shruti_session::transport::Transport;

const SAMPLE_RATE: u32 = 48000;
const BUFFER_SIZE: u32 = 1024;
const CHANNELS: u16 = 2;

/// Generate a 1-second stereo sine wave as an AudioBuffer for the audio pool.
fn sine_source(freq: f32) -> AudioBuffer {
    let frames = SAMPLE_RATE as usize;
    let mut data = Vec::with_capacity(frames * CHANNELS as usize);
    for i in 0..frames {
        let sample =
            (2.0 * std::f32::consts::PI * freq * i as f32 / SAMPLE_RATE as f32).sin() * 0.5;
        for _ in 0..CHANNELS {
            data.push(sample);
        }
    }
    AudioBuffer::from_interleaved(data, CHANNELS)
}

/// Set up N audio tracks each with a region covering at least BUFFER_SIZE frames.
fn setup(num_tracks: usize) -> (Vec<Track>, AudioPool) {
    let mut pool = AudioPool::new();

    // Insert one source per track (different frequencies for variety)
    let mut tracks = Vec::with_capacity(num_tracks);
    for i in 0..num_tracks {
        let freq = 220.0 + i as f32 * 50.0;
        let file_id = format!("source_{i}");
        pool.insert(file_id.clone(), sine_source(freq));

        let mut track = Track::new_audio(format!("Track {i}"));
        track.add_region(Region::new(file_id, 0u64, 0u64, SAMPLE_RATE as u64));
        tracks.push(track);
    }

    (tracks, pool)
}

// ── 4-track render ──────────────────────────────────────────────────

fn timeline_render_4_tracks(c: &mut Criterion) {
    let (tracks, pool) = setup(4);
    let transport = Transport::new(SAMPLE_RATE);
    let mut timeline = Timeline::new(CHANNELS, BUFFER_SIZE);

    c.bench_function("timeline_render_4_tracks", |b| {
        b.iter_batched(
            || AudioBuffer::new(CHANNELS, BUFFER_SIZE),
            |mut output| {
                timeline.render(&tracks, &transport, &pool, &mut output);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── 16-track render ─────────────────────────────────────────────────

fn timeline_render_16_tracks(c: &mut Criterion) {
    let (tracks, pool) = setup(16);
    let transport = Transport::new(SAMPLE_RATE);
    let mut timeline = Timeline::new(CHANNELS, BUFFER_SIZE);

    c.bench_function("timeline_render_16_tracks", |b| {
        b.iter_batched(
            || AudioBuffer::new(CHANNELS, BUFFER_SIZE),
            |mut output| {
                timeline.render(&tracks, &transport, &pool, &mut output);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Groups ──────────────────────────────────────────────────────────

criterion_group!(
    timeline_benches,
    timeline_render_4_tracks,
    timeline_render_16_tracks,
);

criterion_main!(timeline_benches);
