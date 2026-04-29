# Shruti Roadmap — Path to MVP v1

> **Version**: 2026.3.21 | **Last Updated**: 2026-03-21
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
| — Hoosh Full Inclusion | Inference gateway wiring | hoosh 0.20.4 from crates.io; AgentApi hoosh client; MCP `shruti_models` tool; transcribe/describe actions; `/api/models` endpoint; `list_available_models()`; dead code cleanup (ModelManager, prepare_transcription) |
| — Full Dhvani Integration | Complete audio engine | dhvani 0.20.4 with midi+graph features; 9 new DSP effects (biquad, graphic EQ, de-esser, modulated delay, oscillator, LFO, envelope, gain smoother, free functions); 5 analysis wrappers (chromagram, onset, STFT, R128 loudness, silence); clock, MIDI, graph, metering modules; buffer utilities (resample, format conversion, mixing) |

---

## Next Release — Hoosh Remaining

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Prompt-based AI direction | Medium | "play a walking bass line" → hoosh LLM → parse response → MidiToken sequence → AiPlayer |
| 2 | Token budget integration | Small | Per-session token pools via hoosh `TokenBudget`; expose in settings/preferences |
| 3 | Model selection UI | Medium | Dropdown of available models from hoosh `list_models()`; display in AI Player track settings |
| 4 | Streaming generation | Large | Use hoosh `infer_stream()` SSE for real-time token-by-token generation; update InferenceScheduler to consume stream |

---

## Post-MVP

### Synthesizers → Migrated to dhvani

**All synthesis engines now live in dhvani (shared audio crate).** Shruti consumes dhvani's synthesis via feature flags. No duplicate code across consumers. See [dhvani roadmap](https://github.com/MacCracken/dhvani) for full synthesis engine specs.

**Migration plan**:
1. Migrate shruti-instruments synthesis core (subtractive synth, oscillator, voice management, filter, envelope, LFO, mod matrix, drum machine, sampler) into dhvani
2. shruti-instruments becomes a thin layer: preset management, UI parameter mapping, DAW-specific wiring over dhvani engines
3. New synthesis engines (FM, additive, wavetable, physical modeling, granular, vocoder, voice synth) are built directly in dhvani — shruti gets them for free

**What stays in shruti**: InstrumentNode trait, InstrumentPreset (JSON), DAW-specific instrument UI (editors, piano roll, rack), step sequencer UI, plugin hosting (VST3/CLAP). Everything that's DAW, not DSP.

**What moves to dhvani**: Oscillator, envelope, LFO, SVF filter, mod matrix, voice manager, effect chain, drum synthesis, sampler engine. Everything that's DSP, not DAW.

| # | Engine | Location | Shruti Role |
|---|--------|----------|-------------|
| 1 | Subtractive synth | **dhvani** (migrated from shruti-instruments) | UI + presets |
| 2 | FM synth | **dhvani** (new) | UI + presets |
| 3 | Additive synth | **dhvani** (new) | UI + presets |
| 4 | Wavetable synth | **dhvani** (new) | UI + presets |
| 5 | Physical modeling | **dhvani** (new) | UI + presets |
| 6 | Granular synth | **dhvani** (new) | UI + presets |
| 7 | Vocoder | **dhvani** (new) | UI + presets |
| 8 | Drum synth | **dhvani** (migrated from shruti-instruments) | UI + step sequencer |
| 9 | Sampler engine | **dhvani** (migrated from shruti-instruments) | UI + zone editor |
| 10 | Voice synth (formant) | **dhvani** (new) | Vocoder UI in DAW |

**Other consumers that benefit**: jalwa (synthesis effects), kiran/joshua (game audio + NPC voices), vansh (TTS), SY (agent speech), hoosh (audio response mode)

### Goonj Integration (room acoustics for mixing)

- [ ] **Room simulation reverb**: Use `goonj::impulse::generate_ir()` to simulate virtual room acoustics for mixing; expose room dimensions + material as reverb plugin parameters
- [ ] **Room analysis display**: Show `goonj::analysis` metrics (C50, C80, D50, STI) in mixer to help users assess reverb quality
- [ ] **Binaural monitoring**: Use `goonj::binaural::generate_binaural_ir()` for headphone-based spatial monitoring of virtual room placement
- [ ] **Absorption advisor**: Use `goonj::analysis::suggest_absorption_placement()` to recommend acoustic treatment for virtual mix rooms
- [ ] **Coupled studio rooms**: Use `goonj::coupled::coupled_room_decay()` for live room + control room simulation
- [ ] **FDN reverb plugin**: Use `goonj::fdn::Fdn` as real-time reverb effect with room-derived parameters
- [ ] **Ambisonics bus**: Use `goonj::ambisonics::BFormatIr` for spatial reverb sends
- [ ] **Speaker directivity**: Use `goonj::directivity::DirectivityPattern` for monitor placement simulation

### MIDI 2.0

**Goal:** Full MIDI 2.0 (UMP) support per the MIDI Association specification.

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | MIDI-CI (Capability Inquiry) | Medium | Profile negotiation, property exchange between devices |
| 2 | Property exchange | Medium | JSON-based device/plugin property queries |
| 3 | MIDI 2.0 device I/O | Large | Platform MIDI 2.0 drivers (ALSA sequencer, CoreMIDI, WinRT MIDI) |

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

Issues identified in code audit, triaged as Medium/Low. Critical/High issues were fixed inline.

### Performance (from hot-path audit)

| # | Crate | Issue | Est. Impact | Notes |
|---|-------|-------|-------------|-------|
| P1 | shruti-instruments | Synth per-voice buffer writes: accumulate locally, single write at frame end | 5-10% | Reduce 4→2 memory ops per sample per voice |
| P2 | shruti-dsp | Compressor: LUT-based dB conversion instead of per-sample exp2/log2 | 5-8% | 256-entry lookup table for -80..+20 dB |
| P3 | shruti-instruments | Oscillator: cache detuned frequency in voice (update only when detune changes) | 2-4% | Avoids fast_exp2_f64 per sample |
| P4 | shruti-instruments | Unison: precompute detune ratios per buffer (static during block) | 10-20% when active | 8x fast_exp2_f64 per sample → 8x per buffer |
| P5 | shruti-instruments | Envelope: cache stage duration (attack_samples etc.) at trigger, not per-tick | 1-2% | Avoids per-sample division |
| P6 | shruti-dsp | Compressor: specialize compute_gain_db for knee_db=0 vs >0 | 1-3% | Eliminate branch in inner loop |
| P7 | shruti-instruments | Drum machine: only call update_pan_gains when pan changes | <1% | Skip sin/cos per buffer when unnecessary |
| P8 | shruti-instruments | Sampler: cache grain_size from param (update only on param change) | <1% | Minor but consistent pattern |
| P9 | shruti-dsp | All effects: add denormal flushing (flush-to-zero) in filter/reverb feedback loops | Variable | Prevents 50-100x slowdown on silence/reverb tails |

### Architecture & Refactoring

| # | Crate | Issue | Notes |
|---|-------|-------|-------|
| A1 | shruti-instruments | Decompose synth.rs render_voices (~350 lines) into SynthVoice::render, OscillatorMix, FilterChain | Testability + readability |
| A2 | shruti-session | Panics in track.rs enum matching → return Result | 3 panic sites in TrackKind accessors |
| A3 | shruti-session | Session/Track public fields → encapsulate mutable collections with setter validation | API safety |
| A4 | shruti-session | session.rs (1857 lines), track.rs (~1000+ lines) → split at logical boundaries | Monolithic files |
| A5 | shruti-instruments | mem::take workaround for effect_chain → redesign closure API | 4 occurrences across synth/sampler/drum |
| A6 | all | Add #[must_use] to all public functions returning non-unit types | ~50+ functions need it |
| A7 | shruti-session | Improve error specificity: typed variants instead of String wrappers | SessionError granularity |
| A8 | shruti-session | Vec<&Track> returns → impl Iterator in session filtering methods | Avoids allocation per query |
| A9 | all | Audit manual impl Default → derive(Default) where possible | 29 files with manual impl |
| A10 | shruti-instruments | Consider SmallVec/ArrayVec for fixed-size collections (voices, effects) | Avoids heap for small arrays |

---

## Crate Architecture

| Crate | Purpose | Status |
|-------|---------|--------|
| `shruti-engine` | Real-time audio engine, cpal backend, lock-free graph, MIDI I/O (midir) | Active |
| `shruti-dsp` | Audio buffers, format types, file I/O, 14 effects, analysis (FFT, R128, chromagram, onset, STFT), MIDI, graph, clock, metering | Active |
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

*Last Updated: 2026-03-21*
