# Contributing to Shruti

## Getting Started

1. Fork and clone the repo
2. Install Rust 1.85+ via [rustup](https://rustup.rs)
3. Install platform audio libraries:
   - **Linux (Arch/AGNOS):** `sudo pacman -S alsa-lib pipewire-audio`
   - **Linux (Debian):** `sudo apt install libasound2-dev`
   - **macOS:** `xcode-select --install`
   - **Windows:** Visual Studio Build Tools (C++ workload)
4. `cargo build` to verify everything compiles
5. `cargo test` to run the test suite

## CI Checks

All of these must pass before merging:

```sh
cargo fmt --check        # Formatting
cargo clippy --workspace # Lints (0 warnings required)
cargo test --workspace   # 1381+ tests
cargo audit              # No known vulnerabilities
```

## Code Guidelines

- **Real-time safety:** Audio thread code must never allocate, block on a mutex, or perform I/O. Use `try_lock` with fallback, pre-allocated buffers, and lock-free atomics.
- **Type safety:** Use domain newtypes (`FramePos`, `TrackSlot`, `TrackId`, `RegionId`) instead of raw primitives. Use typed parameter enums (`SynthParam`, etc.) instead of magic indices.
- **Typed errors:** Each crate has its own error enum. Never use `Box<dyn Error>` or `String` for errors in library code.
- **No `unwrap()` in library code** — Use `Result`/`Option` propagation. `unwrap()` is acceptable only in tests.
- **Format and lint:** Run `cargo fmt` and `cargo clippy` before committing.
- **Tests:** Add tests for new functionality. DSP code should include accuracy tests with known reference signals.

## Versioning

CalVer: `YYYY.M.D` or `YYYY.M.D-N` for same-day patches. Bump via `./bump-version.sh <version>`.

## Commit Messages

Use conventional commits:
```
feat(engine): add lock-free graph swap
fix(dsp): correct EQ coefficient calculation at high sample rates
refactor(session): replace u64 with FramePos newtype
docs: update architecture overview
```

## Pull Requests

- Keep PRs focused — one feature or fix per PR
- Include a description of what changed and why
- Reference related issues
- Ensure CI passes before requesting review

## Architecture Decisions

Significant design decisions are documented as ADRs in `docs/decisions/`. If your change involves a new architectural pattern or a major trade-off, add an ADR.
