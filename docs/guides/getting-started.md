# Getting Started

## Prerequisites

- **Rust 1.75+** — Install via [rustup](https://rustup.rs)
- **Platform audio libraries** — See below
- **Git** — For cloning the repo

### Linux (AGNOS / Arch)

```sh
sudo pacman -S alsa-lib pipewire-audio jack2
```

### Linux (Debian / Ubuntu)

```sh
sudo apt install libasound2-dev libpipewire-0.3-dev libjack-jackd2-dev
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
# Run the DAW
cargo run --release

# Run tests
cargo test

# Run with verbose logging
RUST_LOG=debug cargo run --release
```

## Project Layout

| Path | Description |
|------|-------------|
| `crates/shruti-engine/` | Real-time audio engine |
| `crates/shruti-dsp/` | DSP primitives and effects |
| `crates/shruti-plugin/` | Plugin hosting (VST3, CLAP) |
| `crates/shruti-ui/` | GPU-accelerated UI |
| `crates/shruti-session/` | Project and session management |
| `crates/shruti-ai/` | AI agent integration (AGNOS) |
| `src/` | Main application entry point |
| `docs/` | Documentation |
