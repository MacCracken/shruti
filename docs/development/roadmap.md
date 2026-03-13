# Shruti Roadmap — Path to MVP v1

> **Version**: 2026.3.11 | **Last Updated**: 2026-03-11
> **Status**: Phases 1–7C, 7A, 7B, 8A–8D complete — MVP v1 + instruments + audit fixes
> **Tests**: 723 passing (168 dsp, 55 engine, 120 instruments, 137 session, 19 plugin, 103 ai, 121 ui), 59.4% line coverage (excl. vendor), 0 clippy warnings, 0 audit vulnerabilities

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
| — Live Recording | Audio capture | Input stream wiring, start/stop recording, buffer→pool→region pipeline, configurable RecordingConfig (44.1–192 kHz, 1–8 ch) |
| 8A — Instrument Engine | Built-in instruments | `shruti-instruments` crate, InstrumentNode trait, VoiceManager, Oscillator (PolyBLEP), ADSR Envelope, SubtractiveSynth |
| — Code Audit (R1-6) | Security, perf, memory, correctness, concurrency | Pre-allocated audio buffers, filter coeff caching, FFT validation, path traversal guard, export overflow guard, record buffer cap, transport loop fix, Acquire/Release atomics, atomic session update |

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

### Live Looped Recording

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Loop-aware overdub recording | Medium | When loop mode is active and recording, each loop iteration creates a new take/layer on armed tracks |
| 2 | Take/layer management | Medium | Stack, mute, solo, delete individual takes per track per loop pass |
| 3 | Comp editing | Large | Select best sections across takes to build a composite region |

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
| 2 | Modulation matrix | Medium | Assignable mod sources (LFO, envelope, velocity, aftertouch, mod wheel) → any parameter; per-voice and global |
| 3 | Effects per instrument | Small | Built-in chorus, distortion, filter drive — reuse existing DSP crate effects where possible |
| 4 | Oscillator anti-aliasing | Done | PolyBLEP for alias-free saw/square at all frequencies |

### 8B+ — Post-MVP Synthesizers

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | FM synth | Large | 4–6 operator FM, algorithm selection (classic DX7-style: 32 algorithms), ratio/detune/feedback per operator, FM matrix routing, velocity→operator level scaling |
| 2 | Additive synth | Large | 64–256 harmonic partials with individual amplitude envelopes, spectral editing (draw/morph), resynthesis from audio (FFT→partials), real-time partial manipulation |
| 3 | Wavetable synth | Large | Wavetable loading (.wav frames, single-cycle), wavetable morphing (smooth interpolation between frames), position modulation via LFO/envelope, built-in factory tables (analog, digital, vocal, organic) |
| 4 | Physical modeling synth | Large | Karplus-Strong string model, waveguide resonators (plucked/bowed/struck), exciter types (noise burst, impulse, bow), body resonance modeling, material parameters (brightness, decay, stiffness) |
| 5 | Granular synth | Large | Grain cloud engine (position, density, size, pitch, spread), real-time granulation of loaded samples, freeze/scatter/spray modes, per-grain envelope (Gaussian/trapezoid), stereo grain panning |
| 6 | Multi-oscillator expansion | Medium | 2–3 oscillators per voice with independent waveform/detune/level, hard sync, ring modulation, oscillator FM (osc1→osc2 cross-mod) |
| 7 | Unison & voice stacking | Medium | Per-oscillator unison voices (up to 8), spread (detune + stereo width), sub-oscillator (-1/-2 octave), supersaw-style detuned stacks |
| 8 | Vocoder | Large | 16–32 band analysis/synthesis filter bank, carrier (synth oscillator or noise) + modulator (mic/audio input), band envelope followers, sibilance detection, formant shift, unvoiced noise injection, freeze mode |

### 8C — Drum Machine

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Drum pad engine | Done | 16-pad sample player, one-shot/looped, pitch/gain/pan/decay, GM drum map (note 36+), velocity sensitivity |
| 2 | Step sequencer | Done | 16/32/64-step grid per pad, swing, per-step probability, accent, BPM-synced |
| 3 | Pattern system | Medium | Pattern banks (A/B/C/D × 16), pattern chaining, song mode (pattern sequence on timeline) |
| 4 | Kit management | Small | Drum kits as preset bundles (samples + tuning + FX); import/export, factory kits |
| 5 | Sample layering | Medium | Velocity layers per pad (up to 8 layers), round-robin, random variation |
| 6 | Per-pad effects | Small | Filter, drive, send to reverb/delay per individual pad |

### 8D — Sampler

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Multi-sample engine | Done | Key zones + velocity zones, root key, pitch ratio, 16-voice polyphony, linear interpolation |
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
| 1 | `TrackKind::Instrument` | Done | New track kind with instrument_type field, `Session::add_instrument_track()`, instrument_params per track |
| 2 | `TrackKind::DrumMachine` | Medium | Specialized instrument track: drum pad layout, step sequencer, pattern-based workflow |
| 3 | `TrackKind::Sampler` | Medium | Specialized instrument track: multi-sample zones, slice mode, time-stretch |
| 4 | `TrackKind::AiPlayer` | Medium | AI-controlled instrument track: model selection, style/creativity params (see Phase 9) |
| 5 | Track kind icons & colors | Small | Distinct icons and default colors per track kind in headers and mixer strips |
| 6 | Track templates | Small | Save/load track configurations (kind + instrument + effects chain + routing) as reusable templates |
| 7 | Track groups / folders | Done | Collapsible track groups with undo/redo, arrangement + mixer UI integration |
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

## MVP v1 Release

Phases 1–6 complete. Phase 7 and MIDI 2.0 follow as post-MVP milestones.

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

### Strategy Notes

- **UI code (841 lines, 22%)** is the biggest gap but hardest to test. Most is egui `Ui` painting code that can't be unit tested without a headless egui context. Focus on extracting and testing the *math and state* behind widgets, not the rendering.
- **shruti-plugin host.rs** needs a `MockPluginInstance` implementing `PluginInstance` to test load/unload/save_state/load_state flows without real shared libraries.
- **cpal_backend** (94 lines, 53%) can be tested by implementing `AudioHost` and `AudioStream` traits on mock structs that capture callback invocations.
- **Diminishing returns** start around 85%: remaining uncovered lines are mostly error branches in I/O code, platform-specific conditionals, and egui draw calls that are impractical to unit test. Integration/screenshot testing would be needed beyond 80%.

*Last Updated: 2026-03-11 — 7-round audit complete, coverage push complete*
