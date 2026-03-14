# Getting Started

## Prerequisites

- **Rust 1.85+** — Install via [rustup](https://rustup.rs)
- **Platform audio libraries** — See below
- **Git** — For cloning the repo

### Linux (AGNOS / Arch)

```sh
sudo pacman -S alsa-lib pipewire-audio
```

### Linux (Debian / Ubuntu)

```sh
sudo apt install libasound2-dev
```

### macOS

```sh
xcode-select --install
```

### Windows

Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload.

## Building

```sh
git clone git@github.com:MacCracken/shruti.git
cd shruti
cargo build --release
```

## Running

```sh
# Launch the DAW
cargo run --release

# Headless playback
cargo run --release --bin shruti-play -- file.wav

# HTTP server for agent integration
cargo run --release -- serve --port 8050

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace
```

## Crate Layout

| Crate | Description |
|-------|-------------|
| `shruti-engine` | Real-time audio engine, cpal backend, lock-free graph, MIDI I/O |
| `shruti-dsp` | Audio buffers, format I/O (WAV/FLAC/AIFF/OGG), DSP effects, metering |
| `shruti-session` | Session, tracks, regions, timeline, transport, undo/redo, preferences |
| `shruti-plugin` | Plugin hosting: CLAP, VST3, native Rust |
| `shruti-instruments` | Built-in instruments: subtractive synth, drum machine, sampler |
| `shruti-ui` | GPU-accelerated DAW UI (egui + eframe) |
| `shruti-ai` | Agent API, MCP tools, analysis, voice control |
| `src/` | Main binary entry point and CLI |
| `tests/` | Cross-crate integration tests |
| `docs/` | Architecture, guides, decisions, development docs |

## Documentation

| Path | Contents |
|------|----------|
| `docs/architecture/` | System design: [overview](../architecture/overview.md), [audio engine](../architecture/audio-engine.md) |
| `docs/decisions/` | Architecture Decision Records (ADRs) |
| `docs/development/` | [Contributing](../development/contributing.md), [roadmap](../development/roadmap.md) |
| `docs/guides/` | This getting started guide |
