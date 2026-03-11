# Audio Engine Design

## Overview

The audio engine is Shruti's real-time core. It processes an audio graph each buffer cycle with hard real-time constraints — every buffer must be filled before the audio device deadline or an xrun (underrun) occurs.

## Real-Time Constraints

The audio thread follows strict rules:

1. **No allocation** — All buffers are pre-allocated. The graph uses arena allocation patterns.
2. **No locks** — Communication with non-RT threads uses lock-free SPSC/MPSC ring buffers.
3. **No I/O** — No file reads, no network, no logging on the RT thread.
4. **No syscalls** — Avoid anything that could invoke the kernel scheduler.

## Audio Graph

The graph is a DAG (directed acyclic graph) of processing nodes:

- **Source nodes** — Audio file playback, input device capture, synth plugins
- **Effect nodes** — EQ, compressor, reverb, delay, plugin effects
- **Routing nodes** — Mixer, splitter, sends
- **Sink nodes** — Output device, bounce-to-disk

### Graph Compilation

The non-RT thread compiles the graph into a flat execution plan (topologically sorted node list). This plan is swapped into the RT thread atomically via a triple-buffer scheme:

```
Non-RT thread:              RT thread:
  build new plan ──swap──▶ read current plan
                            process nodes in order
                            write output buffer
```

The RT thread always has a valid, consistent plan. No partial updates are ever visible.

## Buffer Management

- Fixed buffer size (e.g., 256 or 512 frames), configured at stream creation
- Sample rate matches the audio device (44.1kHz, 48kHz, 96kHz, etc.)
- Audio buffers are `&mut [f32]` slices from a pre-allocated pool
- Inter-node connections carry buffer references, not copies

## Latency Model

```
Total output latency = buffer_size / sample_rate + device_latency
Total roundtrip latency = 2 × buffer_size / sample_rate + device_input_latency + device_output_latency
```

Target: <10ms roundtrip at 256 frames / 48kHz (≈5.3ms buffer latency).
