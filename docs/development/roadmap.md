# Shruti Roadmap — Path to MVP v1

> **Version**: 2026.3.18 | **Last Updated**: 2026-03-18
> **Status**: All MVP phases complete (1–8G, 16A) — remaining work is post-MVP (synth expansion, MIDI 2.0, AI instruments, tarang-demux)
> **Tests**: 1316 passing (190 dsp, 113 engine, 433 instruments, 257 session, 94 plugin, 189 ai, 12 test-utils + 28 e2e integration), 0 clippy warnings, 0 audit vulnerabilities

## Vision

Shruti MVP v1 is a functional DAW capable of recording, editing, mixing, and exporting audio with plugin support. It should be usable for real music production, not just a tech demo. Purpose-built as the primary audio workstation for the AGNOS ecosystem.

---

## Completed Phases

| Phase | Goal | Key Deliverables |
|-------|------|-----------------|
| 1 — Foundation | Audio plays reliably | Cargo workspace (6 crates), cpal backends, lock-free audio graph, AudioBuffer, WAV/FLAC I/O, `shruti-play` CLI |
| 2 — Session & Tracks | Multi-track timeline | Session model (SQLite), Track types (Audio/Bus/Master/Midi), region-based timeline, edit commands, transport, undo/redo |
| 3 — Mixing | Signal routing & effects | DSP effects (EQ, compressor, reverb, delay, limiter, panner), metering (peak/RMS/LUFS), sends/returns, automation |
| 4 — Plugin Hosting | Third-party plugins | CLAP/VST3/Native Rust plugin hosts, scanner, state serialization, PluginNode graph integration |
| 5 — UI | GPU-accelerated interface | egui+eframe (wgpu+winit), arrangement/mixer/transport/browser views, 9 custom widgets, theming, keyboard shortcuts |
| 6 — Export & Polish | Production-ready output | Multi-format export (WAV 16/24/32-bit), MIDI tracks, drag-and-drop import, preferences system, error types |
| — Engine↔UI | Playback & actions | AudioEngine (cpal+atomics), 17 actions wired, waveform/automation/MIDI rendering, meter sync |
| — Devices | Interface enumeration | DeviceInfo (channels, sample rates), midir MIDI ports, Settings view, DeviceCache |
| 7A — Agent API | AI agent control | AgentApi (session/tracks/transport/export), 6 MCP tools, daimon integration |
| 7B — Agnoshi | Natural language | 7 intent patterns, translate module, curl bridge |
| 7C — AI Production | Analysis & auto-mix | Spectral FFT, dynamics (peak/RMS/LUFS/crest), auto-mix suggestions, composition analysis, voice control (12 intents) |
| 7D — AGNOS Distribution | OS integration | Takumi + marketplace recipes, sandbox profile, argonaut service (opt-in), aethersafha Wayland embedding, 5 MCP tools, 5 agnoshi intents |
| — Editing & Routing | Interactive arrangement | Track reorder (drag), region move/trim (drag), bus send routing (3-pass render), submixes |
| — Live Recording | Audio capture | Input stream wiring, start/stop recording, buffer→pool→region pipeline, configurable RecordingConfig (44.1–192 kHz, 1–8 ch) |
| — Code Audit (R1-8) | Security, perf, memory, correctness, concurrency | Pre-allocated audio buffers, filter coeff caching, FFT validation, path traversal guard, export overflow guard, record buffer cap, transport loop fix, Acquire/Release atomics, atomic session update |
| — Engineering (Med) | Constants, setters, undo COW, drag UX, PolyBLEP | Named constants in dsp+instruments, consistent setter patterns, Box-based undo history, drag ghost/cursor feedback, 4-point PolyBLEP oscillator |
| — Test Infrastructure | Shared utils, integration tests, coverage | `shruti-test-utils` crate (8 helpers), 5 cross-crate pipeline tests, 30 AI serve.rs tests, StereoPanner reuse |
| 8A — Instrument Engine | InstrumentNode + MIDI routing | InstrumentNode trait, MidiRoute, VoiceManager (poly/mono/legato, voice stealing), InstrumentPreset JSON, per-instrument undo |
| 8B — Synthesizers | Subtractive + modulation | 3-osc PolyBLEP, dual ADSR, SVF filter, dual LFO, mod matrix (8×8), per-instrument effects (5 types), hard sync, ring mod, FM |
| 8C — Drum Machine | Sample-based drums | 16-pad engine, step sequencer (16/32/64), pattern banks (A/B/C/D × 16), kit management, velocity layers, per-pad effects |
| 8D — Sampler | Multi-sample instrument | Key/velocity zones, sample editing (trim/fade/normalize/reverse), slice mode (onset detection), SFZ/SF2 import |
| 8E — Instrument UI | Editors + piano roll | Instrument rack panel, synth/drum/sampler editors, piano roll (128-note, instrument-aware), parameter automation |
| 8F — Track Types | Organization + routing | 5 track kinds (Instrument/DrumMachine/Sampler/AiPlayer + existing), icons/colors, templates, groups/folders, output routing matrix |
| 8G — Instrument Testing | Comprehensive validation | Oscillator/filter/envelope accuracy, polyphony stress, preset roundtrip, sample playback, sequencer timing, MIDI integration |
| 16A — HTTP Server | AGNOS integration | `shruti serve --port 8050`, axum (8 endpoints + health), CORS, 16 async tests |
| — Tarang Integration | Media backend | tarang-audio decoding (MP3/AAC/ALAC/Opus), FLAC export, channel mixing, resampling (linear + sinc), tarang-ai media analysis |
| — CI/CD & Packaging | Build + distribution | GitHub Actions (CI + release), AGNOS Dockerfile, marketplace recipe, tarang stubs for CI, GPL-3.0 license |

---

## Post-MVP

### Tarang Media Backend (Remaining)
- `tarang-demux` for container-aware import (MP4 audio tracks, MKV, WebM)
- **Benefit**: Shared decode/encode codebase with Tazama and AGNOS media player, wider format support, no ffmpeg dep

### Synthesizers

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | FM synth | Large | 4–6 operator FM, algorithm selection (classic DX7-style: 32 algorithms), ratio/detune/feedback per operator, FM matrix routing, velocity→operator level scaling |
| 2 | Additive synth | Large | 64–256 harmonic partials with individual amplitude envelopes, spectral editing (draw/morph), resynthesis from audio (FFT→partials), real-time partial manipulation |
| 3 | Wavetable synth | Large | Wavetable loading (.wav frames, single-cycle), wavetable morphing (smooth interpolation between frames), position modulation via LFO/envelope, built-in factory tables (analog, digital, vocal, organic) |
| 4 | Physical modeling synth | Large | Karplus-Strong string model, waveguide resonators (plucked/bowed/struck), exciter types (noise burst, impulse, bow), body resonance modeling, material parameters (brightness, decay, stiffness) |
| 5 | Granular synth | Large | Grain cloud engine (position, density, size, pitch, spread), real-time granulation of loaded samples, freeze/scatter/spray modes, per-grain envelope (Gaussian/trapezoid), stereo grain panning |
| 6 | Unison & voice stacking | Medium | Per-oscillator unison voices (up to 8), spread (detune + stereo width), sub-oscillator (-1/-2 octave), supersaw-style detuned stacks |
| 7 | Vocoder | Large | 16–32 band analysis/synthesis filter bank, carrier (synth oscillator or noise) + modulator (mic/audio input), band envelope followers, sibilance detection, formant shift, unvoiced noise injection, freeze mode |

### Sampler

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Time-stretching | Large | Granular or phase-vocoder based pitch-independent time stretch; real-time quality |

### Live Looped Recording

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Loop-aware overdub recording | Medium | When loop mode is active and recording, each loop iteration creates a new take/layer on armed tracks |
| 2 | Take/layer management | Medium | Stack, mute, solo, delete individual takes per track per loop pass |
| 3 | Comp editing | Large | Select best sections across takes to build a composite region |

### MIDI 2.0

**Goal:** Full MIDI 2.0 (UMP) support per the MIDI Association specification.

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Universal MIDI Packet (UMP) | Medium | 32/64/96/128-bit message types, message type routing |
| 2 | MIDI-CI (Capability Inquiry) | Medium | Profile negotiation, property exchange between devices |
| 3 | Per-note controllers | Medium | Per-note pitch bend, pressure, CC — higher resolution than MIDI 1.0 |
| 4 | 32-bit velocity & CC resolution | Small | Upgrade from 7-bit (0-127) to 32-bit resolution |
| 5 | Property exchange | Medium | JSON-based device/plugin property queries |
| 6 | MIDI 2.0 device I/O | Large | Platform MIDI 2.0 drivers (ALSA sequencer, CoreMIDI, WinRT MIDI) |
| 7 | Backward compatibility | Small | Transparent MIDI 1.0 ↔ 2.0 translation layer |

### AI Instruments & Players (Phase 9)

**Goal:** AI-driven virtual instruments that can perform, improvise, and accompany — powered by fine-grained music LLMs running locally on AGNOS. Builds on Phase 8's `InstrumentNode` trait and instrument engine.

#### 9A — Music LLM Integration

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Model runtime | Large | Local inference engine (ONNX Runtime or candle) for music-specific LLMs; CPU + GPU (Vulkan/Metal) |
| 2 | Fine-grained music tokenizer | Large | MIDI→token encoding: note, velocity, duration, timing, instrument; compatible with transformer architectures |
| 3 | Model format & loading | Medium | Standard format for Shruti music models (.shruti-model); versioned, includes tokenizer config + weights |
| 4 | Model manager | Medium | Download, cache, validate models; disk quota management; model registry (local + AGNOS marketplace) |
| 5 | Inference scheduling | Medium | Non-blocking inference on background thread; lookahead buffer so generation stays ahead of playback |
| 6 | Temperature / creativity controls | Small | Per-player controls: temperature, top-k, repetition penalty, style adherence |

#### 9B — AI Player Agents

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Player agent framework | Large | Agent that listens to session context (key, tempo, chord progression, other tracks) and generates MIDI in real-time |
| 2 | Style-conditioned generation | Large | Fine-tune models per genre/instrument: jazz piano, fingerstyle guitar, drum patterns, bass lines, orchestral strings |
| 3 | Accompaniment mode | Medium | AI player follows a lead track (human-played); adjusts dynamics, timing, and harmony to complement |
| 4 | Improvisation mode | Medium | Free-form generation within constraints (key, scale, chord changes, energy curve) |
| 5 | Call-and-response | Medium | AI listens to phrases, generates complementary responses; configurable response delay and style |
| 6 | Arrangement-aware generation | Large | AI reads full session context (all tracks, structure markers, mix levels) to make musically coherent decisions |
| 7 | Human-in-the-loop feedback | Medium | Accept/reject/regenerate individual phrases; RL-style feedback loop to refine player behavior per session |

#### 9C — AI Player UI & UX

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | AI player track integration | Medium | Wire `TrackKind::AiPlayer` (from 8F) with model selection, style, and creativity parameters |
| 2 | Generation timeline view | Medium | Visual display of AI-generated MIDI in arrangement; edit/override individual notes post-generation |
| 3 | Real-time generation indicator | Small | Visual feedback during live generation: confidence level, lookahead buffer status, model activity |
| 4 | Prompt-based direction | Medium | Natural language prompts: "play a walking bass line", "add jazz chords", "build energy into the chorus" |
| 5 | Model training UI | Large | In-app fine-tuning: feed MIDI files as training data, configure epochs/lr, monitor loss, export model |
| 6 | A/B comparison | Small | Generate multiple takes, audition side-by-side, pick or blend |

#### 9D — AI Testing & Validation

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Tokenizer unit tests | Medium | Round-trip MIDI↔token encoding, edge cases (overlapping notes, high velocity, zero-length) |
| 2 | Inference latency benchmarks | Medium | Measure token generation speed vs. playback buffer; CI regression tests for performance |
| 3 | Musical quality metrics | Large | Automated evaluation: rhythmic consistency, harmonic correctness, melodic contour analysis |
| 4 | Model compatibility tests | Small | Validate .shruti-model loading across versions; forward/backward compat |
| 5 | Integration tests | Medium | Full pipeline: MIDI input → tokenizer → model → MIDI output → InstrumentNode → audio |
| 6 | Stress tests | Medium | Multiple AI players simultaneously; measure CPU/GPU/memory under load |
| 7 | Human evaluation framework | Medium | Blind A/B test harness for subjective quality comparison; exportable results |

---

## Engineering Backlog

All CRITICAL/HIGH/MEDIUM issues resolved. Remaining LOW items grouped by domain.

### DSP

| Pri | Item | Notes |
|-----|------|-------|
| L | Zero-copy `as_interleaved()` | Ensure no unnecessary copy in hot audio path |

### Instruments

| Pri | Item | Notes |
|-----|------|-------|
| L | InstrumentPreset clone overhead | Use `Cow` or `Arc` for shared preset data |

### Session

| Pri | Item | Notes |
|-----|------|-------|
| L | SmallString for Track names | Interning or SmallString for hot-path string fields |

### UI / UX

| Pri | Item | Notes |
|-----|------|-------|
| **H** | **UI logic extraction refactor** | Extract state mutations and computation from egui view callbacks into standalone pure functions; enables unit testing of ~2484 lines currently untestable; target files: app.rs, arrangement.rs, mixer.rs, transport.rs, instrument editors |
| L | Theme JSON validation | Reject malformed theme files gracefully |

### Code Quality

| Pri | Item | Notes |
|-----|------|-------|
| M | Test coverage to 70%+ | At 64.3% (5473/8512); blocked by UI rendering — unblocked by UI logic extraction above |

---

## Crate Architecture

| Crate | Purpose | Status |
|-------|---------|--------|
| `shruti-engine` | Real-time audio engine, cpal backend, lock-free graph, MIDI I/O (midir) | Active |
| `shruti-dsp` | Audio buffers, format types, file I/O, effects, metering | Active |
| `shruti-session` | Session, tracks, regions, timeline, transport, undo, MIDI, preferences | Active |
| `shruti-plugin` | Plugin hosting: CLAP, VST3, native Rust | Active |
| `shruti-ui` | GPU-accelerated DAW UI (egui + eframe) | Active |
| `shruti-ai` | Agent API + MCP tools for AGNOS | Active |
| `shruti-instruments` | Built-in instruments: synths, drum machine, sampler, InstrumentNode trait | Active |
| `shruti-test-utils` | Shared test helpers: sine generation, RMS, silence detection | Active |
| `shruti-ml` | Music LLM runtime, tokenizer, AI player agents | Planned |

---

## Test Coverage

**Current:** 1316 tests, 64.3% line coverage (5473/8512 lines, excluding vendor and binaries).
**Tool:** `cargo tarpaulin` with `tarpaulin.toml`.
**CI threshold:** 50% (fails build if coverage drops below).

### Per-Crate Status

| Crate | Coverage | Lines | Remaining gap |
|-------|----------|-------|---------------|
| shruti-dsp | 96.9% | 622/642 | 20 lines — meter LUFS edge cases, limiter |
| shruti-session | 95.4% | 1088/1140 | 52 lines — store error paths, add_track variants |
| shruti-instruments | 94.0% | 2014/2142 | 128 lines — drum looped playback, sampler loop modes |
| shruti-ai | 94.1% | 703/747 | 44 lines — serve.rs run_server, media_analysis |
| shruti-engine | 82.5% | 288/349 | 61 lines — cpal_backend (hardware), midi_io |
| shruti-plugin | 76.3% | 235/308 | 73 lines — LoadedPlugin (needs libloading::Library) |
| shruti-ui | 15.7% | 491/3128 | 2637 lines — egui rendering (not unit-testable) |
| shruti-test-utils | 100% | 34/34 | — |

### Coverage Ceiling Analysis

The UI crate contains 2484 lines of egui rendering code that cannot be unit tested. This caps the theoretical maximum overall coverage at ~71%. Reaching 70%+ requires extracting pure logic from egui view functions into testable helpers.

### Path to 70%+

| Phase | Target | Focus | Strategy |
|-------|--------|-------|----------|
| UI data extraction | 68% | Extract state update logic from egui callbacks | Move mixer/arrangement state mutations into pure functions |
| UI widget extraction | 70% | Extract layout math from widget painting | Separate computation from egui Painter calls |

*Last Updated: 2026-03-18*
