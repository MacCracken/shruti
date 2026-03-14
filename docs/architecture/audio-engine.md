# Audio Engine Design

## Overview

The audio engine is Shruti's real-time core. It processes an audio graph each buffer cycle with hard real-time constraints — every buffer must be filled before the audio device deadline or an xrun (underrun) occurs.

## Real-Time Constraints

The audio callback follows strict rules:

1. **No allocation** — All buffers are pre-allocated. Node outputs, bus buffers, and scratch buffers are created at initialization and reused.
2. **No blocking locks** — Session data and graph plans use double-buffered `try_lock` with fallback. On contention, the previous data is used — never silence.
3. **No I/O** — No file reads, no network, no logging in the normal path. Only `eprintln!` on error recovery (poisoned mutex, buffer mismatch).
4. **No syscalls** — Avoid anything that could invoke the kernel scheduler.

## Audio Graph

The graph is a DAG (directed acyclic graph) of processing nodes:

- **Source nodes** — `FilePlayerNode` (audio file playback with mono upmix and looping)
- **Effect nodes** — `GainNode`, and externally: EQ, compressor, reverb, delay, limiter
- **Plugin nodes** — VST3, CLAP, native Rust plugins via `PluginInstance` trait

### Graph Compilation

The non-RT thread compiles the graph into a flat `ExecutionPlan` (topologically sorted node list via Kahn's algorithm). This plan is published to the RT thread via a double-buffered swap:

```
Non-RT thread:              RT thread:
  build ExecutionPlan        try_lock pending slot
  lock pending slot          if new plan: swap into current_plan
  place plan ──────────▶     else: use existing current_plan
                             process nodes in order
                             copy last node output to device buffer
```

The RT thread always has a valid plan. On `try_lock` contention, it renders using the previous plan — no silence gaps.

## Session Data Sharing

Track and audio pool data uses the same double-buffer pattern:

```
UI thread:                   Audio thread:
  clone tracks + Arc pool     try_lock pending_session
  lock pending_session         if new data: swap into local_data
  place SharedSessionData ──▶  else: use existing local_data
                               build Transport snapshot
                               Timeline::render(tracks, transport, pool, buf)
```

## Transport Synchronization

`SharedTransport` uses lock-free atomics for bidirectional UI/audio sync:

| Field | Type | Writer | Reader | Ordering |
|-------|------|--------|--------|----------|
| `position` | `AtomicU64` | Audio thread (advance) | UI (playhead display) | Acquire/Release |
| `playing` | `AtomicBool` | UI (play/pause/stop) | Audio thread | Acquire/Release |
| `recording` | `AtomicBool` | UI (record toggle) | Audio thread | Acquire/Release |
| `seek_request` | `AtomicU64` | UI (seek) | Audio thread (consume) | AcqRel swap |
| `loop_enabled` | `AtomicBool` | UI (toggle) | Audio thread | Acquire/Release |
| `loop_start` | `AtomicU64` | UI (set loop) | Audio thread | Acquire/Release |
| `loop_end` | `AtomicU64` | UI (set loop) | Audio thread | Acquire/Release |

Seek uses an atomic request slot (`u64::MAX` = no seek pending) so the UI and audio thread never race on `position`.

## Metering

Lock-free peak levels via `AtomicU32` (f32 stored as bit pattern). One stereo pair per track slot plus master. Audio thread writes per callback; UI reads on repaint.

## Recording Pipeline

1. UI calls `start_recording()` — opens cpal input stream with `RecordingConfig` (sample rate, channels, max duration)
2. Input callback appends samples to `Arc<Mutex<Vec<f32>>>` via `try_lock` (drops samples if locked)
3. UI calls `stop_recording()` — drops input stream, drains buffer, returns samples
4. UI creates `AudioBuffer`, inserts into pool, adds `Region` to armed track

## Buffer Management

- Fixed buffer size (e.g., 256 or 512 frames), configured at stream creation
- Sample rate matches the audio device (44.1kHz, 48kHz, 96kHz, etc.)
- Timeline pre-allocates up to 16 bus buffers and 64 source buffers
- `AudioBuffer` uses interleaved storage with per-channel access methods

## Latency Model

```
Output latency    = buffer_size / sample_rate + device_latency
Roundtrip latency = 2 × buffer_size / sample_rate + device_input_latency + device_output_latency
```

Target: <10ms roundtrip at 256 frames / 48kHz (~5.3ms buffer latency).
