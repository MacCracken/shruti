# Shruti Roadmap — Path to MVP v1

> **Version**: 2026.3.13 | **Last Updated**: 2026-03-13
> **Status**: Phases 1–7D, 8A–8G complete — MVP v1 + instruments + full AGNOS integration + audit fixes
> **Tests**: 1136 passing (172 dsp, 55 engine, 339 instruments, 233 session, 19 plugin, 126 ai, 192 ui), 0 clippy warnings, 0 audit vulnerabilities

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
| 8A — Instrument Engine | Built-in instruments | `shruti-instruments` crate, InstrumentNode trait, VoiceManager, Oscillator (PolyBLEP), ADSR Envelope, SubtractiveSynth |
| — Code Audit (R1-6) | Security, perf, memory, correctness, concurrency | Pre-allocated audio buffers, filter coeff caching, FFT validation, path traversal guard, export overflow guard, record buffer cap, transport loop fix, Acquire/Release atomics, atomic session update |

---

## Phase 8: Built-in Instruments

**Goal:** Native virtual instruments — synths, drum machines, samplers — so Shruti is a complete production environment without requiring third-party plugins.

### 8A — Instrument Engine (Complete)

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | `InstrumentNode` trait | Done | Audio graph node: receives MIDI, produces audio; shared interface for all instruments (built-in + AI) |
| 2 | Instrument ↔ MIDI routing | Done | `MidiRoute` with channel filter, note range, velocity curves (Linear/Soft/Hard/Fixed) |
| 3 | Polyphony manager | Done | Voice allocation (mono/poly/legato), voice stealing (oldest/quietest/lowest), configurable max voices |
| 4 | Instrument preset system | Done | `InstrumentPreset` JSON format with save/load, from_instrument/apply_to |
| 5 | Per-instrument undo | Done | `EditCommand::SetInstrumentParam` with full undo/redo |

### 8B — Synthesizers (Complete)

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Subtractive synth | Done | PolyBLEP oscillator, dual ADSR (amp + filter), SVF filter (LP/HP/BP/Notch), dual LFO (6 shapes × 4 targets), 23 params, 16-voice polyphony |
| 2 | Modulation matrix | Done | 8 sources (LFO1/2, AmpEnv, FilterEnv, Velocity, Aftertouch, ModWheel, PitchBend) → 8 destinations, 16 routings, bipolar amounts |
| 3 | Effects per instrument | Done | EffectChain with 5 types (Chorus, Delay, Reverb, Distortion, FilterDrive), integrated into SubtractiveSynth, DrumMachine, Sampler |
| 4 | Oscillator anti-aliasing | Done | PolyBLEP for alias-free saw/square at all frequencies |
| 5 | Multi-oscillator expansion | Done | 3 oscillators per voice with independent waveform/detune/level, hard sync, ring modulation, oscillator FM (osc1→osc2 cross-mod), 16 tests |

### 8C — Drum Machine (Complete)

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Drum pad engine | Done | 16-pad sample player, one-shot/looped, pitch/gain/pan/decay, GM drum map (note 36+), velocity sensitivity |
| 2 | Step sequencer | Done | 16/32/64-step grid per pad, swing, per-step probability, accent, BPM-synced |
| 3 | Pattern system | Done | Pattern banks (A/B/C/D × 16 = 64 patterns), pattern chaining with song mode, copy/select/chain API |
| 4 | Kit management | Done | DrumKit preset: 16-pad config snapshot with save/load JSON, sample_path references, from_drum_machine/apply_to |
| 5 | Sample layering | Done | Velocity layers per pad (up to 8), round-robin/random selection, fallback to main samples |
| 6 | Per-pad effects | Done | PadEffects with one-pole LPF, tanh drive, reverb/delay send levels per pad |

### 8D — Sampler (Complete)

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Multi-sample engine | Done | Key zones + velocity zones, root key, pitch ratio, 16-voice polyphony, linear interpolation |
| 2 | Sample editing | Done | In-place trim, loop points (forward/ping-pong/one-shot), fade in/out, normalize, reverse, peak/RMS analysis |
| 3 | Slice mode | Done | Energy-based onset detection, auto-slice by transients, `slice_to_zones()` maps slices to MIDI keys (REX-style) |
| 4 | Sample format support | Done | WAV, FLAC, AIFF, OGG/Vorbis via symphonia; SUPPORTED_EXTENSIONS, is_supported_extension() |
| 5 | SFZ/SF2 import | Done | SFZ text parser (global/group/region, opcode inheritance, note names) + SF2 binary RIFF parser (preset→instrument→sample zones, PCM extraction), 28 tests |

### 8E — Instrument UI

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Instrument rack panel | Done | Dockable egui panel: instrument selector, parameter knobs grid, preset placeholder, all 4 instrument track kinds |
| 2 | Synth editor | Done | Visual osc (3-osc with enable/detune/level), filter (mode/cutoff/res), ADSR envelopes, dual LFO, FM/sync/ring mod controls |
| 3 | Drum machine grid | Done | 4×4 pad grid with GM drum names, per-pad knobs (pitch/gain/pan/decay), 16-step sequencer, pattern bank selector |
| 4 | Sampler editor | Done | Zone map (128-key keyboard visualization), zone controls (key/vel range, loop mode), waveform placeholder |
| 5 | Piano roll integration | Done | 128-note grid with piano keys, instrument-aware labels (GM drums for DrumMachine), velocity opacity, key range highlighting |
| 6 | Parameter automation | Done | InstrumentParam target variant, label(), instrument_targets() helper |

### 8F — Track Type Organization

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | `TrackKind::Instrument` | Done | New track kind with instrument_type field, `Session::add_instrument_track()`, instrument_params per track |
| 2 | `TrackKind::DrumMachine` | Done | Kit name, pad count (default 16), drum icon, orange color, `add_drum_machine_track()`, 9 tests |
| 3 | `TrackKind::Sampler` | Done | Preset name, zone count, disc icon, teal color, `add_sampler_track()`, 9 tests |
| 4 | `TrackKind::AiPlayer` | Done | Model name, style, creativity (0–1), robot icon, deep purple color, `add_ai_player_track()`, 9 tests |
| 5 | Track kind icons & colors | Done | Unicode icons, RGB default colors, labels per TrackKind; Track::color override with display_color() |
| 6 | Track templates | Done | TrackTemplate: save/load track config (kind, gain, pan, channels, instrument params, color) as JSON |
| 7 | Track groups / folders | Done | Collapsible track groups with undo/redo, arrangement + mixer UI integration |
| 8 | Output routing matrix | Done | Any track → any bus/master; sidechain routing for compressor keying; loop detection via chain walking |

### 8G — Instrument Testing (Complete)

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Oscillator accuracy tests | Done | Frequency accuracy (±1Hz), aliasing measurements, DC offset checks, amplitude consistency across waveforms |
| 2 | Filter response tests | Done | Cutoff accuracy, resonance boost, slope verification, mode switching (LP/HP/BP/Notch) |
| 3 | Envelope timing tests | Done | Attack/decay/release ±1ms at 44100/48000/96000 Hz, sample rate change consistency |
| 4 | Polyphony stress tests | Done | Max voices, voice stealing correctness (oldest/quietest/lowest), allocation under load |
| 5 | Preset roundtrip tests | Done | Synth (with audio verify), DrumMachine, Sampler preset roundtrips + cross-instrument JSON |
| 6 | Sample playback tests | Done | Pitch mapping, loop points (forward/ping-pong), velocity layer selection, one-shot vs gated, drum machine playback |
| 7 | Step sequencer tests | Done | Timing accuracy, swing calculation, probability distribution, pattern chaining, BPM sync |
| 8 | Instrument ↔ MIDI integration | Done | End-to-end: MIDI clip → instrument → audio output for synth, drum machine, sampler |

### 16A — Shruti HTTP Server (Complete)

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | `shruti serve --port 8050` | Done | axum HTTP server wrapping AgentApi (8 endpoints + health), CORS, `Serve` CLI subcommand, 16 async tests |

---

## Post-MVP

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

Items from 6-round code audit. All CRITICAL/HIGH issues fixed. Remaining MEDIUM/LOW grouped by domain, sorted by priority.

### Audio Engine (ui/engine)

| Pri | Item | Notes |
|-----|------|-------|
| H | Lock-free session updates | Replace `Arc<Mutex<SharedSessionData>>` with lock-free triple buffer or crossbeam channel; eliminates `try_lock` silence and `Vec<Track>` clone under lock |
| H | Double-buffered graph plan swap | GraphProcessor should keep stale plan as fallback instead of outputting silence on `try_lock` failure |
| M | Lock-free meter levels | Replace `Arc<Mutex<Vec<[f32;2]>>>` with atomic array or ring buffer — removes lock from both audio callback and UI read |
| M | Mono→stereo channel upmix | FilePlayerNode silently drops right channel for mono sources; duplicate mono to both channels |
| M | Poisoned mutex recovery | Log events; use `into_inner()` to recover instead of silent fallback to silence |
| M | Render failure logging | Add debug logging when interleaved buffer is shorter than expected (currently silent zero-fill) |
| L | DeviceCache diff-based refresh | Diff instead of full rebuild on device enumeration |

### DSP (dsp)

| Pri | Item | Notes |
|-----|------|-------|
| M | EBU R128 compliant LUFS | Current calc divides by `channels*frames`; should average RMS² per channel then convert |
| M | Compressor soft knee verification | Verify formula against standard curves (Fabfilter reference) |
| M | Audio file parsing safety | Wrap symphonia decoding in `catch_unwind` for malformed files |
| L | Reverb/allpass min buffer size | Ensure comb/allpass buffer is at least 1 sample (panic if scale rounds to 0) |
| L | Delay samples explicit clamp | Add `delay_samples.min(buf_len - 1)` instead of relying on implicit modulo behavior |
| L | Zero-copy `as_interleaved()` | Ensure no unnecessary copy in hot audio path |

### Instruments (instruments)

| Pri | Item | Notes |
|-----|------|-------|
| H | Type-safe parameter system | Replace magic number indices (`PARAM_WAVEFORM=0`) with enum-based parameter IDs |
| M | PolyBLEP rising edge correction | Sawtooth only corrects trailing edge; add phase=0 correction for better anti-aliasing |
| M | Envelope stage_pos reset | Reset to 0 on every state transition, not just specific ones |
| M | LFO S&H double-sample at cycle boundary | Move phase advance before S&H check to prevent sampling twice at wrap |
| M | Per-sample `powf` in pitch modulation | Use fast `2^x` approximation (bit manipulation) for real-time pitch bend |
| M | LFO/ADSR helper deduplication | Merge `current_adsr()`/`current_filter_adsr()` into generic; share `lfo_shape_from_param()` |
| M | Sample rate observer trait | Propagate sample rate changes to all child components (oscillator, filter, LFO, envelope) |
| L | InstrumentPreset clone overhead | Use `Cow` or `Arc` for shared preset data |
| L | Filter cutoff modulation docs | Document octave depth mapping for LFO/envelope mod range |

### Session (session)

| Pri | Item | Notes |
|-----|------|-------|
| H | Newtypes for domain IDs | `FramePos(u64)`, `TrackSlot(usize)`, `RegionId(Uuid)` — prevent primitive type confusion |
| M | Audio pool LRU eviction | Keep loaded files in memory up to limit; evict LRU with re-load on demand |
| M | Region list sorted for binary search | Sort by `timeline_pos` for O(log n) `regions_in_range` lookups |
| M | Undo history delta/COW | Current stores full command copies; reduce memory for large sessions |
| M | `VecDeque` for undo stack | Replace `Vec::remove(0)` O(n) with `VecDeque::pop_front()` O(1) |
| M | Schema validation on load | Validate SQLite/JSON session files from untrusted sources |
| L | MidiClip sorted events | Use `BTreeMap` for efficient per-frame MIDI lookup |
| L | SmallString for Track names | Interning or SmallString for hot-path string fields |
| L | Automation dead code cleanup | Remove unreachable `right_idx == 0` check in `value_at()` |

### UI / UX (ui)

| Pri | Item | Notes |
|-----|------|-------|
| **C** | **Auto-save + crash recovery** | Save `.shruti_backup` every 60s; offer recovery on startup; unsaved `*` indicator in title bar |
| **C** | **Background file I/O** | Move save/load/export/audio-pool-load to background thread with progress dialog |
| **C** | **Save prompt on New/Open** | Confirm dialog before discarding unsaved changes |
| H | Error toast notifications | Display user-facing errors for failed operations (currently silent `let _ =`) |
| H | Comprehensive undo/redo | Wrap mute/solo/gain/pan/track-add in `EditCommand`; currently only move/trim are undoable |
| H | Audio engine init feedback | Error dialog if audio device unavailable; offer device selection fallback |
| H | Playhead engine sync | Bidirectional sync between UI transport and `SharedTransport` |
| M | Waveform peaks caching | Cache `WaveformPeaks` per region; invalidate on change only (currently recomputed every frame) |
| M | Snap-to-grid / quantize | Region drag positions quantized to bar/beat grid |
| M | Drag visual feedback | Ghost/opacity on dragged regions; cursor hints on interactive elements |
| M | Recording animation | Blinking red indicator during recording |
| M | Grid level-of-detail | Skip grid lines when closer than 5px at high zoom |
| M | Zoom boundary clamping | Prevent zoom-out making session invisible; handle empty session zoom-to-fit |
| M | Missing keyboard shortcuts | Bind solo, arm, FFwd, export actions to keys |
| M | Audio pool persistence | Save imported/recorded audio alongside session file |
| L | Theme colors caching | Only reapply on theme change (currently re-allocated every `apply_theme()` call) |
| L | Theme JSON validation | Reject malformed theme files gracefully |

### Security (ai/plugin)

| Pri | Item | Notes |
|-----|------|-------|
| M | MCP request size limit | Add max body size in MCP dispatch |
| M | Agent API rate limiting | Throttle MCP/agent endpoints |
| M | Plugin scanner symlink depth | Limit symlink following during directory traversal |
| L | Preferences file permissions | Set 0600 on `preferences.json` |
| L | Plugin state blob validation | Size limit + magic byte check on opaque blobs |
| L | Plugin scanner disk cache | Persist scan results; only re-scan on directory change |

### Code Quality (cross-cutting)

| Pri | Item | Notes |
|-----|------|-------|
| H | Unified `ShrutiError` type | Consistent error handling across all crates; replace mixed `Box<dyn Error>` / `String` |
| M | Shared test utilities crate | Deduplicate `generate_sine()`, `rms_of_buffer()` helpers |
| M | Integration test crate | Cross-crate tests: synth→filter→delay→output pipeline |
| M | Centralize magic numbers | Config module for hardcoded values (window size, max delay, frequency ranges) |
| M | Consistent setter patterns | Standardize on setter methods or public fields in instruments, not both |
| L | Unnecessary `to_vec()` in AI analysis | Pass slice references instead of cloning |
| L | StereoPanner reuse | Reuse panner instances instead of creating per-track per-buffer |

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

## Test Coverage Roadmap (59% → 80%)

**Current:** 723 tests, 59.4% line coverage (2956/4973 lines, excluding vendor).
**Tool:** `cargo tarpaulin` with `tarpaulin.toml` excluding `vendor/*`.

### Per-Crate Status

| Crate | Tests | Coverage | Lines | Gap |
|-------|-------|----------|-------|-----|
| shruti-dsp | 168 | 91% | 595/654 | 59 lines — meter LUFS edge cases, delay stereo, limiter above-ceiling |
| shruti-session | 137 | 95% | 663/699 | 36 lines — session.rs add_track variants, timeline bus overflow |
| shruti-instruments | 120 | 88% | 654/746 | 92 lines — drum_machine looped playback, synth modulation paths, sampler loop modes |
| shruti-engine | 55 | 77% | 208/271 | 63 lines — cpal_backend (needs mock), midi_io enumerate |
| shruti-ai | 103 | 89% | 489/552 | 63 lines — voice.rs command parsing, agent_api error paths |
| shruti-plugin | 19 | 55% | 119/215 | 96 lines — host.rs load/unload/save_state (needs mock PluginInstance) |
| shruti-ui | 121 | 22% | 228/841 | **613 lines** — views, widgets, app.rs, engine.rs, style.rs |
| **Total** | **723** | **59.4%** | **2956/4973** | **2017 lines to 100%** |

### Roadmap to 80% (need ~1022 more lines covered)

| Phase | Target | Lines | Focus | Strategy |
|-------|--------|-------|-------|----------|
| **T1: Low-hanging fruit** | 65% | +275 | shruti-instruments gaps (drum looped, synth mod paths, sampler loops), shruti-plugin mock PluginInstance, shruti-ai voice commands | Unit tests with existing test patterns |
| **T2: Engine mocking** | 70% | +250 | cpal_backend mock (struct implementing AudioHost/AudioStream traits), midi_io with mock midir, engine.rs transport/callback logic | Create `MockBackend` implementing `AudioHost` for test-only use |
| **T3: UI data logic** | 75% | +250 | app.rs action dispatch (extract pure functions from `handle_action`), state.rs transitions, theme/style.rs (test struct construction not rendering), shortcuts.rs | Extract testable logic from egui callbacks; test state machines |
| **T4: UI widget math** | 80% | +250 | fader dB↔linear conversion, knob angle math, meter peak decay, timeline_ruler grid calculation, waveform zoom level selection | Test pure computation functions; skip egui `Ui` painting code |

*Last Updated: 2026-03-13*
