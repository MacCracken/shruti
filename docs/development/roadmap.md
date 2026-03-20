# Shruti Roadmap — Path to MVP v1

> **Version**: 2026.3.20 | **Last Updated**: 2026-03-20
> **Status**: All MVP phases complete (1–8G, 16A) — remaining work is post-MVP (synth expansion, MIDI 2.0, AI instruments)
> **Tests**: 1963 passing, 0 clippy warnings, 0 audit vulnerabilities

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
| — Tarang Integration | Media backend | tarang-audio decoding (MP3/AAC/ALAC/Opus + MP4/MKV/WebM containers), FLAC/Opus/AAC export, channel mixing, resampling (linear + sinc), loudness normalization, streaming decode, tarang-ai media analysis (fingerprint, AcoustID, diarization), container probing |
| — CI/CD & Packaging | Build + distribution | GitHub Actions (CI + release), AGNOS Dockerfile, marketplace recipe, GPL-3.0 license |
| — Crates.io Migration | Tarang + ai-hwaccel | Switched tarang from path deps (3 sub-crates) to unified `tarang 0.19.3` from crates.io; added `ai-hwaccel 0.19.3` with hardware detection module + MCP tool; removed CI stub script |
| — Tarang 0.20.3 Upgrade | Dependency upgrade + 8 features | Upgraded tarang + ai-hwaccel to 0.20.3; container-aware import (MP4/MKV/WebM); cached+selective+async hardware detection; live GPU metrics; AcoustID fingerprinting; speaker diarization; streaming decode; loudness normalization; Opus/AAC export support |
| — Unison & Voice Stacking | Supersaw + sub-osc | Per-osc unison (1-8 voices), detune spread, stereo width, sub-oscillator (-1/-2 oct), 7 new SynthParam variants |
| — Loop Recording & Takes | Overdub + take management | LoopRecordManager with NAN-sentinel splitting, RecordingMode enum, TakeStack/Take structs with mute/solo/delete, transport loop_iteration tracking, AdvanceResult, 3 undo/redo edit commands |
| — Time-Stretching | Granular OLA | Pitch-independent time-stretch (0.25x–4.0x) via dual-grain overlap-add with Hann windows, configurable grain size (10–100ms) |
| — Comp Editing | Take compositing | CompSection-based comp building from TakeStack, build_comp/build_comp_split/build_comp_from_active, CreateComp edit command with undo/redo |
| — MIDI 2.0 / UMP | High-res MIDI + per-note expression | UMP message types, 16/32-bit NoteOnV2/ControlChangeV2/PitchBendV2, per-note pitch bend/pressure/brightness, MIDI 1.0↔2.0 translation, CC processing (mod wheel, brightness), CcMapping |
| 9A — Music LLM Integration | `shruti-ml` crate | MidiTokenizer (584-token vocab, MIDI↔token), ModelRuntime trait + StubRuntime, InferenceScheduler (lookahead buffer), ModelManager (.shruti-model), GenerationConfig |
| 9B — AI Player Agents | AiPlayer InstrumentNode | 3 playback modes (Improvisation/Accompaniment/CallAndResponse), 5 AiPlayerParam controls, sine placeholder rendering, note-on triggers generation |
| — Hoosh Integration | AI inference gateway | HooshRuntime (real LLM via hoosh server), transcription pipeline (Whisper STT), LLM content description, feature-gated `hoosh` |

---

## Next Release — Hoosh Full Inclusion

When hoosh 0.20.3 is published to crates.io, switch from path deps to versioned:

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Switch hoosh to crates.io | Small | `hoosh = { path = "..." }` → `hoosh = { version = "0.20.3", optional = true }` in shruti-ml and shruti-ai |
| 2 | Real music LLM inference | Medium | Replace StubRuntime default with HooshRuntime; wire model selection UI to hoosh `list_models()` |
| 3 | Whisper transcription pipeline | Medium | Wire `transcribe_audio()` to MCP tool and agent API; add vocal alignment via word timestamps |
| 4 | LLM content description | Small | Wire `describe_audio()` to MCP `shruti_analysis` tool for AI-powered audio tagging |
| 5 | Prompt-based AI direction | Medium | "play a walking bass line" → hoosh LLM → parse response → MidiToken sequence → AiPlayer |
| 6 | Token budget integration | Small | Per-session token pools via hoosh `TokenBudget`; expose in settings/preferences |
| 7 | Model selection UI | Medium | Dropdown of available models from hoosh `list_models()`; display in AI Player track settings |
| 8 | Streaming generation | Large | Use hoosh `infer_stream()` SSE for real-time token-by-token generation; update InferenceScheduler to consume stream |

---

## Post-MVP

### ~~Tarang Media Backend (Remaining)~~ (Complete)
- ~~`tarang-demux` for container-aware import (MP4 audio tracks, MKV, WebM)~~ — Done in 2026.3.20
- **Benefit**: Shared decode/encode codebase with Tazama and AGNOS media player, wider format support, no ffmpeg dep

### Synthesizers

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | FM synth | Large | 4–6 operator FM, algorithm selection (classic DX7-style: 32 algorithms), ratio/detune/feedback per operator, FM matrix routing, velocity→operator level scaling |
| 2 | Additive synth | Large | 64–256 harmonic partials with individual amplitude envelopes, spectral editing (draw/morph), resynthesis from audio (FFT→partials), real-time partial manipulation |
| 3 | Wavetable synth | Large | Wavetable loading (.wav frames, single-cycle), wavetable morphing (smooth interpolation between frames), position modulation via LFO/envelope, built-in factory tables (analog, digital, vocal, organic) |
| 4 | Physical modeling synth | Large | Karplus-Strong string model, waveguide resonators (plucked/bowed/struck), exciter types (noise burst, impulse, bow), body resonance modeling, material parameters (brightness, decay, stiffness) |
| 5 | Granular synth | Large | Grain cloud engine (position, density, size, pitch, spread), real-time granulation of loaded samples, freeze/scatter/spray modes, per-grain envelope (Gaussian/trapezoid), stereo grain panning |
| ~~6~~ | ~~Unison & voice stacking~~ | ~~Medium~~ | ~~Per-oscillator unison voices (up to 8), spread (detune + stereo width), sub-oscillator (-1/-2 octave), supersaw-style detuned stacks~~ — Done in 2026.3.20 |
| 7 | Vocoder | Large | 16–32 band analysis/synthesis filter bank, carrier (synth oscillator or noise) + modulator (mic/audio input), band envelope followers, sibilance detection, formant shift, unvoiced noise injection, freeze mode |

### Sampler

| # | Item | Effort | Notes |
|---|------|--------|-------|
| ~~1~~ | ~~Time-stretching~~ | ~~Large~~ | ~~Granular or phase-vocoder based pitch-independent time stretch; real-time quality~~ — Done in 2026.3.20 (granular OLA) |

### Live Looped Recording

| # | Item | Effort | Notes |
|---|------|--------|-------|
| ~~1~~ | ~~Loop-aware overdub recording~~ | ~~Medium~~ | ~~When loop mode is active and recording, each loop iteration creates a new take/layer on armed tracks~~ — Done in 2026.3.20 |
| ~~2~~ | ~~Take/layer management~~ | ~~Medium~~ | ~~Stack, mute, solo, delete individual takes per track per loop pass~~ — Done in 2026.3.20 |
| ~~3~~ | ~~Comp editing~~ | ~~Large~~ | ~~Select best sections across takes to build a composite region~~ — Done in 2026.3.20 |

### MIDI 2.0

**Goal:** Full MIDI 2.0 (UMP) support per the MIDI Association specification.

| # | Item | Effort | Notes |
|---|------|--------|-------|
| ~~1~~ | ~~Universal MIDI Packet (UMP)~~ | ~~Medium~~ | ~~32/64/96/128-bit message types, message type routing~~ — Done in 2026.3.20 |
| 2 | MIDI-CI (Capability Inquiry) | Medium | Profile negotiation, property exchange between devices |
| ~~3~~ | ~~Per-note controllers~~ | ~~Medium~~ | ~~Per-note pitch bend, pressure, CC — higher resolution than MIDI 1.0~~ — Done in 2026.3.20 |
| ~~4~~ | ~~32-bit velocity & CC resolution~~ | ~~Small~~ | ~~Upgrade from 7-bit (0-127) to 32-bit resolution~~ — Done in 2026.3.20 |
| 5 | Property exchange | Medium | JSON-based device/plugin property queries |
| 6 | MIDI 2.0 device I/O | Large | Platform MIDI 2.0 drivers (ALSA sequencer, CoreMIDI, WinRT MIDI) |
| ~~7~~ | ~~Backward compatibility~~ | ~~Small~~ | ~~Transparent MIDI 1.0 ↔ 2.0 translation layer~~ — Done in 2026.3.20 |

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

## ~~Engineering Backlog~~ (Complete)

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
| `shruti-ml` | Music LLM runtime, tokenizer, AI player agents | Active |

---

## Test Coverage

**Current:** 1963 tests (excluding vendor, binaries, and egui rendering).
**Tool:** `cargo tarpaulin` with `tarpaulin.toml`.
**CI threshold:** 70% (fails build if coverage drops below).

Pure egui rendering files (~2500 lines across 16 view/widget files) are excluded from coverage measurement — they contain only `fn(&mut Ui)` callbacks with no extractable logic. All testable computation has been extracted into `logic.rs`, widget test modules, or standalone pure functions.

*Last Updated: 2026-03-20*
