# Architecture Overview

## Design Principles

1. **Real-time safety** — The audio thread never allocates, locks, or blocks. All communication with the audio thread uses lock-free queues and atomic operations.
2. **Modular crates** — Each subsystem is a standalone crate. The engine can be used without the UI; DSP primitives can be used without the engine.
3. **Zero-copy where possible** — Audio buffers are passed by reference through the graph. Copies only happen at explicit boundaries (bounce, export).
4. **Crash isolation** — Plugins run in separate processes. A crashing plugin never takes down the DAW.

## Crate Dependency Graph

```
shruti (binary)
├── shruti-ui
│   ├── shruti-session
│   │   ├── shruti-engine
│   │   │   └── shruti-dsp
│   │   └── shruti-engine
│   └── shruti-engine
├── shruti-plugin
│   └── shruti-engine
└── shruti-ai (optional, AGNOS)
    └── shruti-session
```

## Audio Engine

The audio engine is the core real-time component. It compiles a directed acyclic graph (DAG) of audio nodes into an execution plan that the audio thread processes each buffer cycle.

Key properties:
- **Lock-free graph updates** — The non-RT thread builds a new execution plan and swaps it atomically. The RT thread always has a valid plan.
- **Fixed buffer size** — Buffer size is set at initialization. All nodes process the same number of frames per cycle.
- **Sample-accurate events** — MIDI and automation events carry sample offsets within the buffer for sub-buffer timing.

## Session Model

A session is the top-level project container:
- **Tracks** — Ordered list of audio/MIDI/bus tracks
- **Timeline** — Regions placed on tracks with non-destructive edits
- **Mixer state** — Gain, pan, sends, plugin chains per track
- **Automation** — Parameter curves bound to any automatable parameter
- **Undo history** — Command log enabling full undo/redo

Sessions serialize to a directory containing a SQLite database (metadata, automation, undo history) and a pool of audio files.

## Platform Abstraction

Audio backend selection is compile-time (feature flags) and runtime (user preference):

| Platform | Backends |
|----------|----------|
| Linux | ALSA, PipeWire, JACK |
| macOS | CoreAudio |
| Windows | WASAPI |

A thin `AudioHost` trait abstracts device enumeration, stream creation, and callback registration.
