# Architecture Overview

## Design Principles

1. **Real-time safety** — The audio thread never allocates or blocks. Communication with the audio thread uses double-buffered data slots, lock-free atomics, and `try_lock` with fallback (never silence on contention).
2. **Modular crates** — Each subsystem is a standalone crate. The engine can be used without the UI; DSP primitives can be used without the engine.
3. **Type safety** — Domain newtypes (`FramePos`, `TrackSlot`, `TrackId`, `RegionId`) prevent primitive type confusion. Instrument parameters use typed enums (`SynthParam`, `SamplerParam`, `DrumMachineParam`).
4. **Typed errors** — Each crate has its own error enum (`AudioError`, `SessionError`, `EngineError`, `PluginError`, `InstrumentError`) with proper `Display`, `Error`, and `From` impls.

## Crate Dependency Graph

```
shruti (binary)
├── shruti-ui
│   ├── shruti-session
│   ├── shruti-engine
│   │   └── shruti-dsp
│   └── shruti-plugin
├── shruti-instruments
│   └── shruti-dsp
└── shruti-ai
    ├── shruti-session
    └── shruti-dsp
```

## Audio Engine

The audio engine compiles a directed acyclic graph (DAG) of audio nodes into an execution plan that the audio thread processes each buffer cycle.

Key properties:
- **Double-buffered graph swap** — The non-RT thread builds a new execution plan and places it in a pending slot. The RT thread picks it up via `try_lock`; on contention, it renders the previous plan as fallback.
- **Double-buffered session data** — Track/pool updates use the same pattern: the audio thread owns a local copy and checks for pending updates each cycle.
- **Lock-free transport** — Playhead position, play/pause/record state, and loop settings are shared via `Acquire`/`Release` atomics. Seek uses an atomic request slot to avoid write races.
- **Pre-allocated buffers** — Per-node output buffers, bus accumulation buffers, and scratch buffers are allocated once and reused.

## Session Model

A session is the top-level project container:
- **Tracks** — Ordered list with 8 track kinds: Audio, Bus, Master, Midi, Instrument, DrumMachine, Sampler, AiPlayer
- **Timeline** — Regions placed on tracks with non-destructive edits (move, trim, split, fade)
- **Mixer state** — Gain, pan, mute, solo, sends (pre/post-fader), output routing with loop detection
- **Automation** — Parameter curves (Linear, Step, SCurve) bound to track gain, pan, send levels, instrument params
- **Track groups** — Collapsible folders for organizational grouping
- **Undo history** — Command-pattern log (1000-deep VecDeque) enabling full undo/redo
- **Instruments** — Built-in synth, drum machine, sampler with per-instrument effect chains and presets

Sessions serialize to a directory containing a SQLite database (metadata, automation, undo history) and a pool of audio files.

## Platform Abstraction

Audio backend via cpal:

| Platform | Backend |
|----------|---------|
| Linux | ALSA, PipeWire |
| macOS | CoreAudio |
| Windows | WASAPI |

A thin `AudioHost` trait abstracts device enumeration, stream creation, and callback registration. MIDI I/O via midir.
