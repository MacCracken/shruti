# Shruti

> A Rust-native Digital Audio Workstation

**Version:** 2026.3.14 | **Tests:** 1381 passing | **Clippy:** 0 warnings

**Shruti** is a cross-platform DAW built from the ground up in Rust, designed for performance, reliability, and AI-assisted music production. While OS-independent, Shruti is purpose-built as the primary audio workstation for the [AGNOS](https://github.com/MacCracken/agnosticos) ecosystem.

## Why Shruti?

In Indian classical music, a *shruti* is the smallest interval of pitch the human ear can perceive — the atomic unit of musical expression. This DAW carries that philosophy: precision at every level, from sample-accurate timing to low-latency real-time processing.

## Features

- **Real-time audio engine** — Lock-free, zero-allocation audio graph with double-buffered plan swap
- **Cross-platform** — Linux (ALSA/PipeWire), macOS (CoreAudio), Windows (WASAPI) via cpal
- **Multi-track session** — Audio, MIDI, Bus, Master, Instrument, DrumMachine, Sampler, AiPlayer track types
- **Built-in instruments** — Subtractive synth (34 params, 3 oscillators, SVF filter, dual LFO, mod matrix), drum machine (16 pads, step sequencer, pattern banks), sampler (multi-zone, SFZ/SF2 import)
- **DSP effects** — Parametric EQ, compressor, reverb, delay, limiter, stereo panner, per-instrument effect chains
- **Plugin hosting** — VST3, CLAP, and native Rust plugin API with scanner and state persistence
- **GPU-accelerated UI** — egui + eframe (wgpu/winit) with arrangement, mixer, instrument editor, piano roll, and settings views
- **Non-destructive editing** — Region-based timeline with full undo/redo (1000-deep command history)
- **AI-native workflows** — Agent API, MCP tools, spectral/dynamics analysis, auto-mix suggestions, voice control
- **HTTP server** — `shruti serve` for headless agent integration
- **Modular architecture** — Use as a full DAW or embed individual crates

## Architecture

```
┌──────────────────────────────────────────────────┐
│                   Shruti DAW                     │
├──────────┬───────────┬───────────┬───────────────┤
│ UI Layer │ Session & │ Instrument│ AI Integration│
│(GPU-accel│ Project   │ Engine    │ (AGNOS agents)│
├──────────┴───────────┴───────────┴───────────────┤
│              Audio Engine (real-time)             │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │
│  │ Timeline │ │ Graph    │ │ Plugin Host      │  │
│  │ Renderer │ │ Processor│ │ (VST3/CLAP/Rust) │  │
│  └──────────┘ └──────────┘ └──────────────────┘  │
├──────────────────────────────────────────────────┤
│           Platform Audio Backend (cpal)           │
│        ALSA / PipeWire / CoreAudio / WASAPI       │
└──────────────────────────────────────────────────┘
```

## Crates

| Crate | Description |
|-------|-------------|
| `shruti-engine` | Real-time audio engine, cpal backend, lock-free graph, MIDI I/O |
| `shruti-dsp` | Audio buffers, format I/O (WAV/FLAC/AIFF/OGG), DSP effects, metering |
| `shruti-session` | Session, tracks, regions, timeline, transport, undo/redo, MIDI, preferences |
| `shruti-plugin` | Plugin hosting: CLAP, VST3, native Rust |
| `shruti-instruments` | Built-in instruments: subtractive synth, drum machine, sampler |
| `shruti-ui` | GPU-accelerated DAW UI (egui + eframe) |
| `shruti-ai` | Agent API, MCP tools, analysis, voice control |

## Building

```sh
# Prerequisites: Rust 1.85+, system audio dev libraries
# Linux (Arch/AGNOS): sudo pacman -S alsa-lib pipewire-audio
# Linux (Debian):     sudo apt install libasound2-dev
# macOS:              xcode-select --install
# Windows:            Visual Studio Build Tools (C++ workload)

cargo build --release
cargo run --release
```

## CLI

```sh
shruti                    # Launch the DAW
shruti play file.wav      # Headless playback
shruti serve --port 8050  # HTTP server for agent integration
```

## CI

```sh
cargo fmt --check
cargo clippy --workspace
cargo test --workspace
cargo audit
```

## Versioning

CalVer: `YYYY.M.D` or `YYYY.M.D-N` for same-day patches. See [CHANGELOG](CHANGELOG.md).

## AGNOS Integration

On AGNOS, Shruti exposes its full capabilities to AI agents through a structured API and MCP tools:

- Voice-driven session control (12 intent categories)
- Automated mixing and mastering (spectral analysis, gain staging, pan spread)
- Real-time audio analysis (FFT, dynamics, LUFS)
- HTTP server for headless agent workflows

## License

GPLv3 — See [LICENSE](LICENSE) for details.
