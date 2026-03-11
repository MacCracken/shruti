# Shruti

> A Rust-native Digital Audio Workstation

**Shruti** is a cross-platform DAW built from the ground up in Rust, designed for performance, reliability, and AI-assisted music production. While OS-independent, Shruti is purpose-built as the primary audio workstation for the [AGNOS](https://github.com/MacCracken/agnosticos) ecosystem.

## Why Shruti?

In Indian classical music, a *shruti* is the smallest interval of pitch the human ear can perceive — the atomic unit of musical expression. This DAW carries that philosophy: precision at every level, from sample-accurate timing to low-latency real-time processing.

## Goals

- **Real-time audio engine** — Lock-free, zero-allocation audio graph with deterministic latency
- **Cross-platform** — Linux (ALSA/PipeWire/JACK), macOS (CoreAudio), Windows (WASAPI)
- **Plugin support** — VST3, CLAP, and native Rust plugin API
- **AI-native workflows** — Agent-driven composition, mixing, and mastering (first-class on AGNOS)
- **Non-destructive editing** — Immutable audio graph with full undo history
- **Modular architecture** — Use as a full DAW or embed individual crates (engine, DSP, UI)

## Architecture

```
┌──────────────────────────────────────────────────┐
│                   Shruti DAW                     │
├──────────────┬──────────────┬────────────────────┤
│   UI Layer   │  Session &   │   AI Integration   │
│  (GPU-accel) │  Project Mgmt│   (AGNOS agents)   │
├──────────────┴──────────────┴────────────────────┤
│              Audio Engine (real-time)             │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │
│  │ Mixer    │ │ Graph    │ │ Plugin Host      │  │
│  │          │ │ Compiler │ │ (VST3/CLAP/Rust) │  │
│  └──────────┘ └──────────┘ └──────────────────┘  │
├──────────────────────────────────────────────────┤
│           Platform Audio Backend                 │
│   ALSA / PipeWire / JACK / CoreAudio / WASAPI    │
└──────────────────────────────────────────────────┘
```

## Project Structure

```
shruti/
├── crates/
│   ├── shruti-engine/     # Real-time audio engine
│   ├── shruti-dsp/        # DSP primitives and effects
│   ├── shruti-plugin/     # Plugin hosting (VST3, CLAP)
│   ├── shruti-ui/         # GPU-accelerated UI
│   ├── shruti-session/    # Project/session management
│   └── shruti-ai/         # AI agent integration
├── src/                   # Main application binary
└── docs/                  # Documentation
```

## Building

```sh
# Prerequisites: Rust 1.75+, system audio dev libraries
# Linux: libasound2-dev (ALSA) or pipewire-dev
# macOS: Xcode command line tools
# Windows: Visual Studio Build Tools

cargo build --release
cargo run --release
```

## AGNOS Integration

On AGNOS, Shruti exposes its full capabilities to AI agents through a structured API, enabling:

- Voice-driven session control
- Automated mixing and mastering pipelines
- Generative composition with agent collaboration
- Real-time audio analysis and feedback

## License

GPLv3 — See [LICENSE](LICENSE) for details.
