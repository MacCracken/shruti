# Shruti Roadmap — Path to MVP v1

> **Version**: 2026.3.11 | **Last Updated**: 2026-03-11
> **Status**: Phases 1–7C, 7A, 7B, 8A complete — MVP v1 + instruments in progress
> **Tests**: 441 passing (66 dsp, 9 engine, 31 instruments, 108 session, 3 plugin, 103 ai, 121 ui), 51% line coverage, 0 clippy warnings, 0 audit vulnerabilities

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
| — Editing & Routing | Interactive arrangement | Track reorder (drag), region move/trim (drag), bus send routing (3-pass render), submixes |
| — Live Recording | Audio capture | Input stream wiring, start/stop recording, buffer→pool→region pipeline |
| 8A — Instrument Engine | Built-in instruments | `shruti-instruments` crate, InstrumentNode trait, VoiceManager, Oscillator (PolyBLEP), ADSR Envelope, SubtractiveSynth |

---

## Phase 7: AGNOS Integration (remaining)

**Goal:** First-class AI agent support on AGNOS.

### 7D — AGNOS Distribution

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Takumi recipe | Done | Build from source, native binary, desktop entry |
| 2 | Marketplace recipe | Done | Auto-version from release tags |
| 3 | Sandbox profile | Done | PipeWire/ALSA, Landlock, Wayland |
| 4 | Argonaut service integration | Small | Optional auto-start in Desktop mode |
| 5 | Aethersafha Wayland integration | Medium | Embed in compositor, proper surface management |

**Exit criteria:** An AGNOS agent can open a session, arrange tracks, apply effects, mix, and export — with human oversight.

---

## Post-MVP: MIDI 2.0

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

---

## Phase 8: Built-in Instruments

**Goal:** Native virtual instruments — synths, drum machines, samplers — so Shruti is a complete production environment without requiring third-party plugins.

### 8A — Instrument Engine (Complete)

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | `InstrumentNode` trait | Done | Audio graph node: receives MIDI, produces audio; shared interface for all instruments (built-in + AI) |
| 2 | Instrument ↔ MIDI routing | Medium | MIDI track → InstrumentNode routing, channel filtering, velocity curves, note range splits |
| 3 | Polyphony manager | Done | Voice allocation (mono/poly/legato), voice stealing (oldest/quietest/lowest), configurable max voices |
| 4 | Instrument preset system | Medium | JSON preset format with parameter snapshots, save/load/share, factory preset packs, user presets |
| 5 | Per-instrument undo | Small | Parameter changes are undoable via existing UndoManager |

### 8B — Synthesizers

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Subtractive synth | Done (basic) | Single oscillator with PolyBLEP (saw/square/tri/sine/noise), ADSR envelope, 16-voice polyphony. TODO: multi-osc, filter, LFOs |
| 2 | Wavetable synth | Large | Wavetable loading (.wav frames), wavetable morphing, position modulation, built-in factory tables |
| 3 | FM synth | Large | 4–6 operator FM, algorithm selection (classic DX-style), ratio/detune/feedback per operator, FM matrix |
| 4 | Modulation matrix | Medium | Assignable mod sources (LFO, envelope, velocity, aftertouch, mod wheel) → any parameter; per-voice and global |
| 5 | Effects per instrument | Small | Built-in chorus, distortion, filter drive — reuse existing DSP crate effects where possible |
| 6 | Oscillator anti-aliasing | Medium | PolyBLEP or minBLEP for alias-free waveforms at all frequencies |

### 8C — Drum Machine

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Drum pad engine | Medium | 16-pad sample player, each pad = one-shot or looped sample with pitch/gain/pan/decay |
| 2 | Step sequencer | Medium | 16/32/64-step grid per pad, adjustable swing, probability per step, accent, flam |
| 3 | Pattern system | Medium | Pattern banks (A/B/C/D × 16), pattern chaining, song mode (pattern sequence on timeline) |
| 4 | Kit management | Small | Drum kits as preset bundles (samples + tuning + FX); import/export, factory kits |
| 5 | Sample layering | Medium | Velocity layers per pad (up to 8 layers), round-robin, random variation |
| 6 | Per-pad effects | Small | Filter, drive, send to reverb/delay per individual pad |

### 8D — Sampler

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Multi-sample engine | Large | Key zones + velocity zones, crossfade between zones, root key detection |
| 2 | Sample editing | Medium | In-place trim, loop points (forward/ping-pong/one-shot), fade, normalize |
| 3 | Time-stretching | Large | Granular or phase-vocoder based pitch-independent time stretch; real-time quality |
| 4 | Slice mode | Medium | Auto-slice by transients, map slices to MIDI keys (REX-style) |
| 5 | Sample format support | Small | WAV, FLAC, AIFF, OGG import; leverage existing shruti-dsp I/O |
| 6 | SFZ/SF2 import | Medium | Load SoundFont and SFZ instrument definitions for instant playability |

### 8E — Instrument UI

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Instrument rack panel | Medium | Dockable egui panel: instrument selector, parameter knobs/sliders, preset browser |
| 2 | Synth editor | Large | Visual oscillator, filter, envelope, LFO editors with real-time waveform preview |
| 3 | Drum machine grid | Medium | 16-pad grid view with step sequencer, pattern selector, per-pad waveform display |
| 4 | Sampler editor | Medium | Waveform view with loop points, zone editor (key/velocity matrix), drag-and-drop sample loading |
| 5 | Piano roll integration | Medium | Per-instrument piano roll respects key ranges, drum names on rows for drum tracks |
| 6 | Parameter automation | Small | All instrument parameters exposed as automation targets in arrangement view |

### 8F — Track Type Organization

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | `TrackKind::Instrument` | Medium | New track kind: hosts an InstrumentNode, receives MIDI, outputs audio; instrument selection per track |
| 2 | `TrackKind::DrumMachine` | Medium | Specialized instrument track: drum pad layout, step sequencer, pattern-based workflow |
| 3 | `TrackKind::Sampler` | Medium | Specialized instrument track: multi-sample zones, slice mode, time-stretch |
| 4 | `TrackKind::AiPlayer` | Medium | AI-controlled instrument track: model selection, style/creativity params (see Phase 9) |
| 5 | Track kind icons & colors | Small | Distinct icons and default colors per track kind in headers and mixer strips |
| 6 | Track templates | Small | Save/load track configurations (kind + instrument + effects chain + routing) as reusable templates |
| 7 | Track groups / folders | Medium | Collapsible track folders, group solo/mute, shared bus routing |
| 8 | Output routing matrix | Medium | Any track → any bus/master; sidechain routing for compressor keying |

### 8G — Instrument Testing

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Oscillator accuracy tests | Medium | Frequency accuracy, aliasing measurements, DC offset checks across full MIDI range |
| 2 | Filter response tests | Medium | Verify cutoff, resonance, slope against expected frequency response curves |
| 3 | Envelope timing tests | Small | Attack/decay/release timing accuracy within ±1ms at various sample rates |
| 4 | Polyphony stress tests | Medium | Max voices, voice stealing correctness, no clicks/pops on voice allocation |
| 5 | Preset roundtrip tests | Small | Save/load every factory preset, verify identical output |
| 6 | Sample playback tests | Medium | Correct pitch mapping, loop points, velocity layer selection, one-shot vs gated |
| 7 | Step sequencer tests | Medium | Timing accuracy, swing calculation, probability distribution, pattern chaining |
| 8 | Instrument ↔ MIDI integration | Medium | End-to-end: MIDI clip → instrument → audio output verification |

---

## Post-MVP: AI Instruments & Players

**Goal:** AI-driven virtual instruments that can perform, improvise, and accompany — powered by fine-grained music LLMs running locally on AGNOS. Builds on Phase 8's `InstrumentNode` trait and instrument engine.

### 9A — Music LLM Integration

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Model runtime | Large | Local inference engine (ONNX Runtime or candle) for music-specific LLMs; CPU + GPU (Vulkan/Metal) |
| 2 | Fine-grained music tokenizer | Large | MIDI→token encoding: note, velocity, duration, timing, instrument; compatible with transformer architectures |
| 3 | Model format & loading | Medium | Standard format for Shruti music models (.shruti-model); versioned, includes tokenizer config + weights |
| 4 | Model manager | Medium | Download, cache, validate models; disk quota management; model registry (local + AGNOS marketplace) |
| 5 | Inference scheduling | Medium | Non-blocking inference on background thread; lookahead buffer so generation stays ahead of playback |
| 6 | Temperature / creativity controls | Small | Per-player controls: temperature, top-k, repetition penalty, style adherence |

### 9B — AI Player Agents

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Player agent framework | Large | Agent that listens to session context (key, tempo, chord progression, other tracks) and generates MIDI in real-time |
| 2 | Style-conditioned generation | Large | Fine-tune models per genre/instrument: jazz piano, fingerstyle guitar, drum patterns, bass lines, orchestral strings |
| 3 | Accompaniment mode | Medium | AI player follows a lead track (human-played); adjusts dynamics, timing, and harmony to complement |
| 4 | Improvisation mode | Medium | Free-form generation within constraints (key, scale, chord changes, energy curve) |
| 5 | Call-and-response | Medium | AI listens to phrases, generates complementary responses; configurable response delay and style |
| 6 | Arrangement-aware generation | Large | AI reads full session context (all tracks, structure markers, mix levels) to make musically coherent decisions |
| 7 | Human-in-the-loop feedback | Medium | Accept/reject/regenerate individual phrases; RL-style feedback loop to refine player behavior per session |

### 9C — AI Player UI & UX

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | AI player track integration | Medium | Wire `TrackKind::AiPlayer` (from 8F) with model selection, style, and creativity parameters |
| 2 | Generation timeline view | Medium | Visual display of AI-generated MIDI in arrangement; edit/override individual notes post-generation |
| 3 | Real-time generation indicator | Small | Visual feedback during live generation: confidence level, lookahead buffer status, model activity |
| 4 | Prompt-based direction | Medium | Natural language prompts: "play a walking bass line", "add jazz chords", "build energy into the chorus" |
| 5 | Model training UI | Large | In-app fine-tuning: feed MIDI files as training data, configure epochs/lr, monitor loss, export model |
| 6 | A/B comparison | Small | Generate multiple takes, audition side-by-side, pick or blend |

### 9D — AI Testing & Validation

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
| `shruti-ml` | Music LLM runtime, tokenizer, AI player agents | Planned |

---

## MVP v1 Release

Phases 1–6 complete. Phase 7 and MIDI 2.0 follow as post-MVP milestones.

---

## Test Coverage Roadmap (40% → 80%)

Current: 441 tests, 51% line coverage.

| Milestone | Target | Focus Areas | Est. Tests |
|-----------|--------|-------------|------------|
| 50% | +300 lines | shruti-ui views (arrangement interactions, mixer logic), shruti-engine backend mocking | +40 |
| 60% | +300 lines | shruti-session undo/edit commands (all EditCommand variants), store roundtrips, audio_pool edge cases | +30 |
| 70% | +300 lines | shruti-ui widget interactions (fader drag, knob drag, track selection), shruti-plugin host lifecycle, error recovery paths | +30 |
| 80% | +300 lines | Integration tests (session→timeline→export pipeline), shruti-ai MCP dispatch coverage, binary CLI arg parsing, edge cases across all crates | +30 |

**Strategy:**
- UI rendering code: mock-free tests for data flow and state transitions; skip pixel-level rendering
- Engine code: mock audio backends, test graph execution with synthetic nodes
- Session code: full undo/redo cycle tests for every EditCommand variant
- Plugin code: mock plugin instances, test scanner with fixture directories

*Last Updated: 2026-03-11 (8A complete, recording wired)*
