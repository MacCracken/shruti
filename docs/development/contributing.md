# Contributing to Shruti

## Getting Started

1. Fork and clone the repo
2. Install Rust 1.75+ via [rustup](https://rustup.rs)
3. Install platform audio libraries:
   - **Linux:** `sudo pacman -S alsa-lib pipewire-audio` (Arch/AGNOS) or `sudo apt install libasound2-dev` (Debian)
   - **macOS:** Xcode command line tools (`xcode-select --install`)
   - **Windows:** Visual Studio Build Tools
4. `cargo build` to verify everything compiles
5. `cargo test` to run the test suite

## Code Guidelines

- **Real-time safety:** Code running on the audio thread must never allocate, lock a mutex, or perform I/O. Use `#[deny(unsafe_code)]` in non-engine crates.
- **No `unwrap()` in library code** — Use `Result` or `Option` propagation. `unwrap()` is acceptable only in tests.
- **Format and lint:** Run `cargo fmt` and `cargo clippy` before committing. CI enforces both.
- **Tests:** Add tests for new functionality. DSP code should include accuracy tests with known reference signals.

## Commit Messages

Use conventional commits:
```
feat(engine): add lock-free graph swap
fix(dsp): correct EQ coefficient calculation at high sample rates
docs: update architecture overview
```

## Pull Requests

- Keep PRs focused — one feature or fix per PR
- Include a description of what changed and why
- Reference related issues
- Ensure CI passes before requesting review

## Architecture Decisions

Significant design decisions are documented in `docs/architecture/`. If your change involves a new architectural pattern or a major trade-off, add or update the relevant doc.
