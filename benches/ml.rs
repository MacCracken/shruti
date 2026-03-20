use criterion::{Criterion, criterion_group, criterion_main};
use shruti_ml::model::StubRuntime;
use shruti_ml::{AiPlayer, GenerationConfig, InferenceScheduler, MidiToken, MidiTokenizer};

use shruti_dsp::AudioBuffer;
use shruti_instruments::InstrumentNode;
use shruti_session::midi::NoteEvent;
use shruti_session::types::FramePos;

// ── Tokenizer ──────────────────────────────────────────────────

fn tokenizer_encode(c: &mut Criterion) {
    let tokenizer = MidiTokenizer::new(44100, 120.0, 4);
    let events: Vec<NoteEvent> = (0..100)
        .map(|i| NoteEvent {
            position: FramePos(i * 4410),
            duration: FramePos(2205),
            note: 60 + (i % 12) as u8,
            velocity: 80 + (i % 48) as u8,
            channel: 0,
        })
        .collect();

    c.bench_function("tokenizer_encode_100_notes", |b| {
        b.iter(|| tokenizer.encode(&events));
    });
}

fn tokenizer_decode(c: &mut Criterion) {
    let tokenizer = MidiTokenizer::new(44100, 120.0, 4);
    // Build a realistic token sequence
    let mut tokens = vec![MidiToken::Bos];
    for i in 0..100 {
        tokens.push(MidiToken::TimeShift(25));
        tokens.push(MidiToken::Velocity(16 + (i % 16) as u8));
        tokens.push(MidiToken::Duration(8));
        tokens.push(MidiToken::NoteOn(60 + (i % 12) as u8));
    }
    tokens.push(MidiToken::Eos);

    c.bench_function("tokenizer_decode_100_notes", |b| {
        b.iter(|| tokenizer.decode(&tokens));
    });
}

fn tokenizer_roundtrip(c: &mut Criterion) {
    let tokenizer = MidiTokenizer::new(44100, 120.0, 4);
    let events: Vec<NoteEvent> = (0..50)
        .map(|i| NoteEvent {
            position: FramePos(i * 4410),
            duration: FramePos(2205),
            note: 60 + (i % 12) as u8,
            velocity: 100,
            channel: 0,
        })
        .collect();

    c.bench_function("tokenizer_roundtrip_50_notes", |b| {
        b.iter(|| {
            let tokens = tokenizer.encode(&events);
            tokenizer.decode(&tokens)
        });
    });
}

// ── Inference Scheduler ────────────────────────────────────────

fn scheduler_fill_buffer(c: &mut Criterion) {
    c.bench_function("scheduler_fill_64_tokens", |b| {
        b.iter_batched(
            || {
                let rt = Box::new(StubRuntime::new());
                InferenceScheduler::new(rt, GenerationConfig::default(), 64)
            },
            |mut sched| sched.fill_buffer(),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn scheduler_fill_and_consume(c: &mut Criterion) {
    c.bench_function("scheduler_fill_consume_256_tokens", |b| {
        b.iter_batched(
            || {
                let rt = Box::new(StubRuntime::new());
                InferenceScheduler::new(rt, GenerationConfig::default(), 256)
            },
            |mut sched| {
                sched.fill_buffer();
                while sched.next_token().is_some() {}
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── AI Player ──────────────────────────────────────────────────

fn ai_player_render_1024(c: &mut Criterion) {
    let mut player = AiPlayer::new(48000.0);
    player.start_generation();
    // Warm up — generate initial events
    let mut warmup = AudioBuffer::new(2, 4096);
    player.process(&[], &[], &mut warmup);

    c.bench_function("ai_player_render_1024", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 1024),
            |mut buf| player.process(&[], &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn ai_player_render_256(c: &mut Criterion) {
    let mut player = AiPlayer::new(48000.0);
    player.start_generation();
    let mut warmup = AudioBuffer::new(2, 4096);
    player.process(&[], &[], &mut warmup);

    c.bench_function("ai_player_render_256", |b| {
        b.iter_batched(
            || AudioBuffer::new(2, 256),
            |mut buf| player.process(&[], &[], &mut buf),
            criterion::BatchSize::SmallInput,
        );
    });
}

// ── Token ID conversion ────────────────────────────────────────

fn token_id_roundtrip(c: &mut Criterion) {
    let tokens: Vec<MidiToken> = (0..MidiToken::VOCAB_SIZE)
        .filter_map(MidiToken::from_id)
        .collect();

    c.bench_function("token_id_roundtrip_full_vocab", |b| {
        b.iter(|| {
            for &token in &tokens {
                let id = token.to_id();
                let _ = MidiToken::from_id(id);
            }
        });
    });
}

// ── Groups ──────────────────────────────────────────────────────

criterion_group!(
    tokenizer_benches,
    tokenizer_encode,
    tokenizer_decode,
    tokenizer_roundtrip,
    token_id_roundtrip,
);

criterion_group!(
    scheduler_benches,
    scheduler_fill_buffer,
    scheduler_fill_and_consume,
);

criterion_group!(player_benches, ai_player_render_256, ai_player_render_1024,);

criterion_main!(tokenizer_benches, scheduler_benches, player_benches);
