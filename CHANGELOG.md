# Changelog

All notable changes to Shruti are documented here.
Format: CalVer `YYYY.M.D` or `YYYY.M.D-N` for same-day patches.

## 2026.3.20 — Tarang 0.20.3 Upgrade & New Features

### Dependency Upgrades
- Upgraded `tarang` from 0.19.3 to 0.20.3
- Upgraded `ai-hwaccel` from 0.19.3 to 0.20.3
- Added `async-detect` feature to `ai-hwaccel` for non-blocking hardware probing
- Added `MediaInfo.metadata` field for tarang 0.20.3 compatibility
- Fixed `AudioBuffer::num_samples` → `num_frames` rename across all tarang integration code (mix, resample, writer, media_analysis)

### Container-Aware Import (tarang-demux)
- Added MP4, MOV, MKV, WebM to `SUPPORTED_EXTENSIONS` — audio tracks from video containers can now be imported directly
- New `probe_container()` function: inspects container metadata (codec, sample rate, channels, duration, video presence) without decoding
- New `ContainerInfo` struct for container probe results

### Hardware Detection Improvements
- **Cached detection**: Results cached for 5 minutes via `LazyLock`-based cache with TTL, avoiding repeated subprocess spawns
- **Selective backends**: Only probes CUDA and ROCm (desktop GPU focus) via `DetectBuilder`
- **Async detection**: New `detect_async()` function for non-blocking hardware probing in async contexts (axum server)
- **Live GPU metrics**: `DeviceInfo` now includes `memory_free_bytes`, `memory_used_bytes`, `gpu_utilization_percent`, `temperature_c`, `power_watts`
- **Cache control**: New `invalidate_cache()` function for manual cache reset

### Audio Analysis Expansion
- **AcoustID fingerprinting**: New `compute_acoustid()` — base64-encoded fingerprints compatible with AcoustID music identification database
- **Speaker diarization**: New `diarize_audio()` — energy-based VAD + spectral clustering for who-spoke-when segmentation
- Extracted shared `to_tarang_buffer()` and `parse_codec()` helpers, reducing duplication

### Streaming Audio Decode
- New `StreamingReader` with `open()`, `next_buffer()`, `read_all()` — pull-based streaming decode via `FileDecoder::next_buffer()` for memory-efficient playback of long files
- New `StreamingError` enum (`EndOfStream`, `Decode`)

### Loudness Normalization
- New `loudness` module in `shruti-dsp` (feature-gated: `tarang`):
  - `measure_loudness()` — ITU-R BS.1770 simplified LUFS measurement
  - `apply_gain()` — dB gain with clamping
  - `normalize_loudness()` — two-pass normalize to target LUFS (e.g., -14 for Spotify/YouTube)

### Opus & AAC Export
- Added `Opus` and `Aac` variants to `ExportFormat`
- New feature flags: `opus-enc` (libopus) and `aac-enc` (fdk-aac) forwarded through workspace
- `write_opus_tarang()` — Opus encoding via OGG muxer
- `write_aac_tarang()` — raw AAC encoding via fdk-aac

### Unison & Voice Stacking (Post-MVP)
- **Per-oscillator unison**: 1-8 detuned copies of Osc1 with configurable detune spread (0-100 cents)
- **Stereo spread**: unison voices panned across stereo field
- **Sub-oscillator**: dedicated -1 or -2 octave oscillator with independent waveform and level
- 7 new `SynthParam` variants: `UnisonVoices`, `UnisonDetune`, `UnisonSpread`, `SubOscEnable`, `SubOscOctave`, `SubOscWaveform`, `SubOscLevel`
- Added `unison_phases[8]` and `sub_phase` to Voice struct

### Loop-Aware Overdub Recording (Post-MVP)
- **`LoopRecordManager`**: lock-free ring buffer recording with NAN-sentinel take splitting
- **`RecordingMode`** enum: `Normal`, `Overdub`, `Replace`
- `push_loop_marker()` for RT-safe loop boundary signaling
- `finish_all()` writes separate WAV files per take (e.g., `take_001.wav`, `take_002.wav`)
- Transport now tracks `loop_iteration` counter (resets on stop)
- `AdvanceResult` struct with `loop_wrapped` and `loop_iteration` fields

### Take/Layer Management (Post-MVP)
- New `take` module in `shruti-session`: `Take`, `TakeId`, `TakeStack` structs
- `TakeStack`: add, mute, unmute, solo, delete, set_active, audible_take
- Added `take_stacks: Vec<TakeStack>` to Track (serde-compatible, default empty)
- New edit commands: `SetActiveTake`, `MuteTake`, `DeleteTake` with full undo/redo

### Sampler Time-Stretching (Post-MVP)
- **Granular overlap-add time-stretcher**: pitch-independent speed control (0.25x to 4.0x ratio)
- Two overlapping Hann-windowed grains at 50% overlap for smooth output
- Configurable grain size (10-100ms, default 50ms)
- `TimeStretch` and `GrainSize` parameters added to `SamplerParam` (indices 5-6)
- `source_pos` and `grain_phase` per-voice state for independent time/pitch tracking
- Normal playback path preserved unchanged when time-stretch is 1.0

### Comp Editing (Post-MVP)
- **`CompSection`** struct: time range assigned to a specific take (offset + duration + take_index)
- **`build_comp()`**: converts a list of CompSections into playable Regions on the timeline
- **`build_comp_split()`**: convenience for two-section comps (split at a frame offset)
- **`build_comp_from_active()`**: single-region comp from the active take
- **`CreateComp`** edit command with full undo/redo (adds/removes regions)
- Built entirely on existing Region primitives — no core data structure changes

### Tests
- 1913 tests passing (up from 1847), 0 clippy warnings
- New tests: synth unison/sub-osc (5), take management (13), loop recording (6), transport loop iteration (3), plus AcoustID (2), diarization (2), loudness (5), container probe (2), hardware cache (3), GPU metrics (1)

## 2026.3.19 — Crates.io Migration & Hardware Detection

### Tarang Crates.io Migration
- Replaced 3 path-based tarang sub-crate dependencies (`tarang-core`, `tarang-audio`, `tarang-ai` at `../../../tarang/crates/`) with single unified `tarang = "0.19.3"` from crates.io
- Updated all imports: `tarang_core::` → `tarang::core::`, `tarang_audio::` → `tarang::audio::`, `tarang_ai::` → `tarang::ai::`
- Removed `scripts/create-tarang-stubs.sh` (no longer needed — tarang resolves from crates.io)
- Removed CI stub creation step from `.github/actions/setup-rust-env/action.yml`

### AI Hardware Acceleration
- Added `ai-hwaccel = "0.19.3"` dependency to `shruti-ai`
- New `hardware` module in `shruti-ai`: wraps `ai-hwaccel` for GPU/NPU/TPU detection
- `HardwareInfo` struct: summarizes detected accelerators, memory, suggested quantization (targeting 7B-param music LLM for Phase 9)
- `detect()` function: queries all available backends via `AcceleratorRegistry`
- New MCP tool `shruti_hardware` with `detect` action — exposes hardware capabilities to AGNOS agents

### Test Coverage to 89.65% (MEDIUM backlog item)
- 140 new tests across all crates: instruments (sfz, sf2, drum_machine), ai (agent_api, serve), session, plugin (host), ui (automation_lane, logic)
- Fixed infinite loop bug in sf2 `ChunkIter` — truncated chunks now advance offset instead of looping
- Updated tarpaulin config: excluded 16 pure egui rendering files (~2500 lines), raised CI threshold from 50% to 70%
- Coverage: 89.65% (5389/6011 testable lines)

### UI Logic Extraction (HIGH backlog item)
- New `logic` module in `shruti-ui` with 8 pure functions extracted from egui callbacks:
  - `fast_forward_position()` — tempo-based bar advancement (from `app.rs`)
  - `pixel_to_frame()` — pixel coordinate to timeline frame conversion (was duplicated 4x in `arrangement.rs`)
  - `calculate_trim_start()` / `calculate_trim_end()` — region trim geometry with bounds checking
  - `row_index_from_y()` — track reorder target calculation
  - `gain_to_db_string()` — linear gain to dB display (from `mixer.rs`)
  - `parameter_changed()` — float epsilon comparison
  - `finalize_recording()` — recording buffer to region conversion (from `app.rs`)
- 27 new unit tests covering all extracted functions
- Updated `app.rs`, `arrangement.rs`, `mixer.rs` to use extracted functions

### Theme JSON Validation (LOW backlog item)
- Added WCAG 2.0 color contrast validation to `Theme::validate()`
- New `relative_luminance()` and `contrast_ratio()` helpers (pub(crate))
- Validates `bg_primary` vs `text_primary` contrast ratio >= 3.0:1 (WCAG AA large text)
- 7 new tests: luminance calculation, contrast ratio, validation pass/fail cases

### Criterion Benchmarks
- New `benches/` directory with 4 benchmark files (criterion 0.5 with HTML reports)
- **dsp.rs** — 8 benchmarks: EQ, compressor, reverb, delay, limiter processing + AudioBuffer ops (from_interleaved, mix_from, apply_gain)
- **instruments.rs** — 4 benchmarks: SubtractiveSynth (256/1024 frames), DrumMachine, Sampler rendering
- **timeline.rs** — 2 benchmarks: 4-track and 16-track mixdown (1024 frames)
- **tarang_audio.rs** — 9 benchmarks (feature-gated `tarang`): WAV read/write, FLAC export (16/24-bit), linear + sinc resampling, channel mixing
- Found 2 tarang 0.19.3 bugs via benchmarks: linear resampler off-by-one on stereo, mix_channels stereo→mono size mismatch (filed on tarang roadmap, fixed in 0.20.3)

### Engineering Backlog Cleanup
- Closed 3 LOW-priority items as resolved/non-issues:
  - `as_interleaved()` — already zero-copy (returns `&[Sample]` directly, verified by pointer-identity test)
  - `InstrumentPreset` clone overhead — no production clones; `apply_to` borrows immutably
  - `SmallString` for track names — not in hot path; lookups by `TrackId`, single-digit track counts

## 2026.3.18 — Test Infrastructure & Backlog Cleanup

### Shared Test Utilities Crate
- New `shruti-test-utils` crate with shared helpers: `generate_sine()`, `rms_of_buffer()`, `has_audio()`, `is_silent()`, `count_zero_crossings()`, `fill_sine()`, `peak_amplitude()`, `collect_note_ons()`
- Deduplicated identical helpers across `tests/integration.rs`, `crates/shruti-instruments/tests/midi_integration.rs`, and `crates/shruti-dsp/src/effects/eq.rs`
- 12 self-tests validating helper correctness

### Integration Test Expansion
- 5 new cross-crate integration tests: synth→EQ→compressor→delay pipeline, drum machine→WAV export roundtrip, sampler multi-zone pitch mapping, track template roundtrip, undo/redo gain/pan via UndoManager
- Total integration tests: 28 root-level + 7 instrument-level

### Coverage Boost (shruti-ai)
- 30 new tests in `serve.rs`: rate limiter edge cases (boundary, window reset, single-RPS), default value functions, serde default integration, CORS 127.0.0.1 origin, add_region/spectrum/dynamics/auto_mix/save/open/redo endpoints, MCP tracks/transport dispatch, constant validation

### StereoPanner Reuse
- `Timeline` now holds a single `StereoPanner` instance, reused across all tracks per render call instead of allocating a new one per-track per-buffer

### CI/CD Overhaul
- New `setup-rust-env` composite action: Rust toolchain install, system deps, cargo caching, tarang stub creation, version extraction
- CI workflow: concurrency groups with cancel-in-progress for branch pushes
- Dockerfile migrated to AGNOS base image (`ghcr.io/maccracken/agnosticos:latest`) with `ark` package manager
- Docker build uses `--features tarang` for full codec support
- `scripts/create-tarang-stubs.sh` — creates minimal stub crates for CI environments without the tarang repo

### AGNOS Marketplace Recipe
- New `recipes/marketplace/shruti.toml` — AGNOS package definition with runtime/build dependencies, sandbox config (seccomp + Landlock), and marketplace metadata

### License
- Added GPL-3.0 LICENSE file

### Additional Test Coverage
- Agent API: save/open session roundtrip, add_region with pool audio, add_region track-not-found error path
- SharedTransport: seek consume/reset, multiple-seeks-last-wins, playing toggle

## 2026.3.17 — Engineering Backlog (Medium Priority)

### Centralized Constants
- New `constants` module in `shruti-dsp`: DB_FLOOR, LINEAR_FLOOR, LUFS_OFFSET, reverb defaults (Freeverb reference rate, feedback/damping coefficients), delay defaults, compressor defaults, limiter defaults, meter constants (LUFS block duration, peak decay, max blocks), spectral analysis constants (MAX_FFT_SIZE, rolloff threshold)
- New `constants` module in `shruti-instruments`: musical constants (A4_FREQUENCY, A4_MIDI_NOTE, SEMITONES_PER_OCTAVE, CENTS_PER_OCTAVE), filter range (MIN/MAX_CUTOFF_HZ, NYQUIST_RATIO), envelope defaults, polyphony limits, RNG seeds, LFO/sampler constants
- All inline magic numbers replaced with named constants across reverb, delay, compressor, limiter, meter, spectral, dynamics, oscillator, filter, envelope, LFO, voice, synth, sampler
- 8 new tests validating constant ranges and relationships

### Consistent Instrument Setter Patterns
- `DrumPad.pan` made private with `pan()` getter and `set_pan()` setter — automatically recomputes cached stereo gains, eliminating the `update_pan_gains()` footgun
- `Envelope.state` and `Envelope.level` made private with `state()` and `level()` getters — state machine managed only through `trigger()`, `release()`, `reset()`
- All callers updated across drum_machine, drum_kit, synth, and test code

### Undo History Memory Optimization
- Heavy data (`Region`, `TrackGroup`) in `EditCommand` variants boxed via `Box<>` — reduces enum size so lightweight variants (trims, gains, toggles) don't pay the size cost of the largest variant
- Affected variants: AddRegion, RemoveRegion, MoveRegionToTrack, SplitRegion, CreateGroup, RemoveGroup
- Removed TODO(perf) comment — optimization implemented
- New test: `edit_command_enum_is_compact` validates enum stays smaller than Region

### Drag Visual Feedback
- Region move: dashed outline at original position + ghost overlay at dragged position
- Trim operations (start/end): amber outline showing current region bounds during trim
- Grab cursor on region center hover, ResizeHorizontal on edge handles
- Grabbing cursor during active region move and track reorder drags
- Grab cursor on track header hover for reorder affordance

### 4-Point PolyBLEP Anti-Aliasing
- Upgraded oscillator from 2-point to 4-point PolyBLEP (integrated cubic residual)
- Wider correction window (2 samples each side vs 1) for better aliasing suppression at high frequencies
- Output clamped to [-1, 1] to prevent minor overshoot from wider kernel
- 3 new tests: outer sample correction, boundary continuity, high-frequency output range

## 2026.3.16 — Tarang Audio Backend Integration

### Audio Decoding via tarang-audio
- Replaced direct symphonia dependency in shruti-dsp with tarang-audio (`FileDecoder`)
- All audio file reading now goes through tarang-audio's decode pipeline
- Removed `symphonia` workspace dependency — symphonia is now a transitive dep via tarang
- **New format support**: MP3, AAC (M4A), ALAC, Opus — in addition to existing WAV, FLAC, AIFF, OGG/Vorbis
- `SUPPORTED_EXTENSIONS` expanded: `mp3`, `m4a`, `aac`, `alac`, `opus`
- Empty file handling: gracefully returns zero-frame buffer when tarang reports no audio decoded
- `catch_unwind` safety wrapper retained for malformed file protection

### Native FLAC Export via tarang-audio
- FLAC encoding via `tarang_audio::FlacEncoder` when `tarang` feature is enabled
- `write_audio_file()` dispatches FLAC to tarang encoder (falls back to WAV without feature)
- `is_native_export()` helper to check if a format has native support

### Channel Mixing via tarang-audio
- New `mix` module in `shruti-dsp::io` — stereo/mono conversion and multichannel downmixing
- Supports stereo→mono, mono→stereo, 5.1→stereo (ITU-R BS.775), generic N-channel fallbacks
- Available with `tarang` feature flag

### Audio Resampling via tarang-audio
- New `resample` module in `shruti-dsp::io` — linear interpolation and windowed sinc resampling
- `resample()` for fast linear interpolation (previews, non-critical paths)
- `resample_sinc()` for high-quality windowed sinc with configurable window size

### Media Analysis via tarang-ai
- New `media_analysis` module in `shruti-ai` — content classification, audio fingerprinting, transcription routing
- `analyze_audio()` — auto-tagging imported samples by content type and quality
- Re-exports tarang-ai types: `AudioFingerprint`, `ContentType`, `MediaAnalysis`, `TranscriptionResult`
- Hoosh client and Whisper model integration for fingerprinting and transcription
- Available with `tarang` feature flag

## 2026.3.14 — Engineering Backlog (High Priority)

### Newtypes for Domain IDs
- `FramePos(u64)` newtype for all frame-based positions and durations — prevents confusion with raw u64 values (buffer sizes, sample counts)
- `TrackSlot(usize)` newtype for track slot indices — prevents confusion with other usize indices
- `#[serde(transparent)]` on both types for backward-compatible JSON serialization
- Arithmetic ops (Add, Sub, Rem, AddAssign, SubAssign), Ord, Hash, Display
- Helper methods: `as_f32()`, `as_f64()`, `abs_diff()`, `saturating_sub()`, `min()`, `max()`
- Migrated all frame fields across 7 crates: Region, Transport, AutomationPoint, MidiClip, NoteEvent, ControlChange, EditCommand, Timeline, Session, ArrangementDrag, Agent API

### Type-Safe Instrument Parameters
- `SynthParam` enum (34 variants) replacing magic number indices for SubtractiveSynth
- `SamplerParam` enum (5 variants) for Sampler
- `DrumMachineParam` enum (1 variant) for DrumMachine
- `ParamIndex` trait with `index()` and `count()` methods
- `From<Enum> for usize` and `TryFrom<usize> for Enum` conversions
- `get_param()`/`set_param()` convenience methods on all instruments
- All internal code uses enum variants; old PARAM_* constants removed

### Unified Error Types
- `EngineError` enum: Backend, Graph, Recording, Io variants — replaces `Box<dyn Error>` in shruti-engine
- `PluginError` enum: NotFound, LoadError, StateError, ScanError, Io — replaces `String` errors in shruti-plugin
- `InstrumentError` enum: ParseError, InvalidConfig, Io — replaces `String` errors in SF2/SFZ parsers
- All error types implement Display, Error (with proper `source()` chains), and relevant From conversions

### Bidirectional Playhead Engine Sync
- UI transport controls (play/stop/pause/seek) now drive audio engine via `SharedTransport` atomics
- Audio thread position advances are read back to UI transport during playback
- `SharedTransport::seek_request` — atomic seek slot avoids UI/audio race on position writes
- `SharedTransport::loop_enabled/loop_start/loop_end` — loop settings shared via atomics
- Audio callback handles loop wrapping with modulo overshoot
- `AudioEngine::sync_transport()` — pushes UI loop settings to audio thread
- 4 new SharedTransport tests (seek request, loop sync, default values)

### Lock-Free Session Updates (Double-Buffered)
- Replaced `Arc<Mutex<SharedSessionData>>` with double-buffered pending slot pattern
- Audio callback owns local copy of session data — no lock needed to read
- UI thread writes to `pending_session: Arc<Mutex<Option<SharedSessionData>>>`
- Audio thread picks up pending data via `try_lock()` on each callback
- On contention, audio thread continues rendering with previous data instead of outputting silence
- Eliminates `try_lock` silence gaps that occurred during session updates

### Double-Buffered Graph Plan Swap
- `GraphProcessor` now stores `current_plan` (owned by RT thread) + `pending_plan` (written by non-RT thread)
- On `try_lock` failure, RT thread uses previous plan as fallback instead of outputting silence
- Poisoned mutex recovery preserved across all code paths
- 3 new tests: fallback rendering on contention, pending plan pickup, empty processor silence

### Test & Quality
- 1381 total tests across workspace (up from 1327)
- 0 clippy warnings
- 0 audit vulnerabilities

---

## 2026.3.11 — Initial Release

### Phase 1: Foundation
- Cargo workspace with 6 crates: engine, dsp, plugin, session, ui, ai
- Cross-platform audio backend via cpal (ALSA/PipeWire, CoreAudio, WASAPI)
- Lock-free audio graph with topological sort and atomic plan swap
- `AudioBuffer` with interleaved storage, per-channel access, mix, gain
- Audio file I/O: WAV/FLAC read (symphonia), WAV write (hound)
- `shruti-play` CLI for headless playback and recording
- `shruti` CLI with device listing

### Phase 2: Session & Tracks
- `Session` model with SQLite persistence and sidecar audio pool
- Track types: Audio, Bus, Master with gain/pan/mute/solo
- Region-based non-destructive timeline with fade in/out
- Edit commands: add, remove, move, split, trim, fade, gain, pan
- Transport: play, pause, stop, loop, seek, tempo, time signature
- Full undo/redo system (command pattern, 1000-deep history)
- Session serialization via `SessionStore` (SQLite + JSON)

### Phase 3: Mixing
- Built-in DSP effects: `ParametricEq` (biquad filters, multi-band), `Compressor` (threshold/ratio/attack/release/knee), `Reverb` (Schroeder/Freeverb-style), `Delay` (stereo with feedback), `Limiter` (brickwall with fast attack), `StereoPanner` (balance control)
- Metering: `Meter` with peak, RMS, and integrated LUFS (EBU R128 gating)
- Sends & returns: `Send` struct with pre/post-fader routing to bus tracks
- Automation: `AutomationLane` with `AutomationPoint`, `CurveType` (Linear, Step, SCurve), per-frame interpolation
- Timeline panning: stereo balance applied per-track during render
- Automation integration: track gain and pan automated from lanes during timeline render

### Phase 5: UI
- GPU-accelerated DAW interface using egui + eframe (wgpu + winit backends)
- Arrangement view: timeline with tracks, region clips, waveform rendering, grid lines, playhead cursor
- Mixer view: channel strips with fader, meter, pan knob, M/S buttons, dB readout
- Transport bar: play/stop/record buttons, time display (hh:mm:ss + bar.beat.tick), BPM drag, loop toggle
- Browser panel: toggleable bottom panel with Files tab (rfd import) and Plugins tab (search filter)
- Custom widgets: `Fader`, `LevelMeter`, `Knob`, `WaveformPeaks`, `TrackHeader`, `TimelineRuler`, `RegionClip`, `AutomationLane`, `PluginSlot`
- Theme system: JSON-serializable `ThemeColors` with 28 named colors, `apply_theme()` styling, `Theme::load()`/`save()`
- Keyboard shortcuts: configurable `ShortcutRegistry` with 25+ default keybindings (Space=Play, Enter=Stop, R=Record, etc.)
- View switcher: Arrangement/Mixer toggle with quick-add track button
- Scroll zoom: Ctrl+scroll for horizontal zoom, shift/trackpad for horizontal scroll

### Phase 6: Export & Polish
- Multi-format export: WAV with Int16, Int24, Float32 bit depth via `ExportConfig`
- `write_audio_file()` dispatcher with configurable `ExportFormat` and `BitDepth`
- `AgentApi::export_audio()` — export with format and bit depth parameters
- MIDI track support: `TrackKind::Midi`, `MidiClip` with `NoteEvent` and `ControlChange`
- `MidiClip` queries: `notes_at()`, `note_ons_at()`, `note_offs_at()` for per-frame lookup
- `Session::add_midi_track()`, `Session::midi_tracks()`
- Drag-and-drop file import: audio files dropped onto arrangement or browser are auto-imported
- Visual drop zone overlay indicator when hovering files
- Preferences system: `Preferences` struct with audio device, sample rate, buffer size, project dir, recent sessions, UI scale, theme path, auto-save interval
- JSON persistence with XDG-aware default paths, `load_or_default()`
- Error types: `AudioError` (I/O, format, decoding, export, buffer mismatch) and `SessionError` (I/O, database, serialization, track/region not found)

### Phase 4: Plugin Hosting
- `PluginInstance` trait — unified API for all plugin formats
- CLAP host — load .clap plugins, verify `clap_entry` symbol
- VST3 host — load .vst3 bundles, platform-aware binary discovery
- Native Rust plugin API — `shruti_plugin_create` entry point
- `PluginScanner` — scan standard paths on Linux/macOS/Windows
- `PluginState` — serializable parameter values + opaque binary chunk
- `PluginNode` — integrate any plugin into the audio graph
- `PluginHost` — manage plugin instances with slot-based activation

### Engine↔UI Integration
- `AudioEngine` — cpal-backed audio output with lock-free transport (`SharedTransport` via atomics) and `SharedSessionData` (Arc<Mutex> with `try_lock` for non-blocking audio thread)
- `Timeline` rendering in audio callback: real-time playback of multi-track sessions
- All 17 keyboard actions wired: undo/redo, cut/copy/paste, delete, split, duplicate, new/open/save/export session, zoom-to-fit, fast-forward, new bus track, toggle arm
- Waveform rendering inside region clips via `WaveformPeaks::from_samples()` + `draw_waveform()`
- Automation lane rendering in arrangement view
- MIDI clip rendering with colored rectangles, note bars, and clip names
- Meter level sync from engine to UI mixer view
- Drag-and-drop loads audio files into `AudioPool` (arrangement + browser)

### Audio & MIDI Device Enumeration
- Enhanced `DeviceInfo` with `max_channels` and `supported_sample_rates` fields
- `CpalBackend` extracts channel counts and sample rates from `supported_output_configs()`/`supported_input_configs()`
- `AudioHost::all_devices()` — merges input/output devices with unified I/O flags
- MIDI port enumeration via `midir`: `enumerate_midi_ports()` returns `MidiPortInfo` (name, direction)
- Settings view: lists audio interfaces (default indicator, I/O direction, channel count, sample rates) and MIDI devices (inputs/outputs)
- `DeviceCache` with on-demand refresh to avoid scanning every frame
- `ViewMode::Settings` — new view accessible from view switcher
- `Session::audio_device_name` field for device preference persistence

### Editing & Routing
- Track reordering: `Session::move_track()`, `swap_tracks()` with master-track protection
- `EditCommand::MoveTrack` with full undo/redo support
- Bus send routing: 3-pass `Timeline::render()` — pre-fader sends, post-fader sends, bus accumulation into master
- `Session::add_send()` / `remove_send()` with bus-track validation
- Interactive arrangement view: region click-to-select with accent highlight border
- Region drag-to-move with live preview and undo integration
- Region trim handles: 5px resize zones at left/right edges, `ResizeHorizontal` cursor, live trim start/end
- Track header drag-to-reorder with visual drop indicator line
- `ArrangementDrag` enum (MoveRegion, TrimStart, TrimEnd, ReorderTrack) for drag state tracking
- Pending action collection pattern to avoid borrow conflicts in egui immediate mode
- 20 new tests: track reorder edge cases (out-of-bounds, same-index, dirty flag), send routing (invalid source, out-of-bounds remove, multiple sends, bus gain, muted track, empty bus), ArrangementDrag state construction, MoveTrack compound undo

### Phase 7C: AI-Assisted Production
- Spectral analysis API: radix-2 FFT, `analyze_spectrum()` returning peak frequency, spectral centroid, spectral rolloff, magnitude spectrum in dB
- Dynamics analysis API: `analyze_dynamics()` returning peak, RMS, true peak (4x oversampled), crest factor, LUFS, dynamic range per channel
- Auto-mix agent: `auto_mix_suggest()` — per-track gain staging (target -18 dBFS RMS), stereo pan spread, EQ suggestions based on spectral centroid
- Composition suggestions: `composition_suggest()` — arrangement structure, instrumentation, tempo recommendations based on session analysis
- Voice control via vansh: `parse_voice_input()` — 12 intent categories (transport, seek, mute/solo, volume, pan, tempo, mix, analysis) with confidence scoring
- MCP tool: `shruti_analysis` — 4 actions (spectrum, dynamics, auto_mix, composition) for agent-driven analysis

### Live Recording
- `AudioEngine::start_recording()` — opens cpal input stream, captures audio to lock-free ring buffer
- `AudioEngine::stop_recording()` — stops input stream, returns captured samples
- `SharedTransport::recording` atomic flag for UI state sync
- Record action wired in UI: arm track → start capture → stop → create `AudioBuffer` → insert into pool → add Region to armed track

### Phase 8A: Instrument Engine
- New `shruti-instruments` crate with 5 modules: instrument, voice, oscillator, envelope, synth
- `InstrumentNode` trait: receives MIDI events (`NoteEvent`, `ControlChange`), produces audio; shared interface for all instruments
- `InstrumentParam`: named parameter with min/max/default, normalized get/set, unit label
- `VoiceManager`: polyphony management with configurable max voices and steal modes (Oldest, Quietest, Lowest, None)
- `Voice`: state machine (Idle/Active/Releasing), MIDI note→frequency conversion, phase accumulator, age tracking
- `Oscillator`: PolyBLEP anti-aliased waveforms (Sine, Saw, Square, Triangle, Noise), detune in cents
- `Envelope`: ADSR generator with per-stage sample counting, trigger/release, smooth release from any level
- `SubtractiveSynth`: 16-voice polyphonic synth — 23 parameters (waveform, amp ADSR, volume, detune, filter cutoff/resonance/mode, filter envelope ADSR + depth, dual LFO with rate/depth/target/shape)
- `Filter`: state-variable filter (Cytomic SVF) with LowPass, HighPass, BandPass, Notch modes
- `Lfo`: 6 shapes (Sine, Triangle, Square, SawUp, SawDown, SampleAndHold), configurable rate/depth
- Dual LFO system: LFO1 + LFO2 with independent targets (None, Cutoff, Pitch, Volume)
- Filter envelope: separate ADSR for filter cutoff modulation with bipolar depth (-1..+1)

### Phase 8A: MIDI Routing & Presets
- `MidiRoute`: channel filter, note range, velocity curve (Linear/Soft/Hard/Fixed)
- `InstrumentPreset`: JSON-serializable parameter snapshots with save/load, `from_instrument()` / `apply_to()`
- `TrackKind::Instrument`: new track kind with instrument_type field, `Session::add_instrument_track()`
- `EditCommand::SetInstrumentParam`: per-instrument parameter undo/redo

### Phase 8B: Synthesizer Expansion
- Multi-mode SVF filter per voice integrated into SubtractiveSynth
- LFO→filter cutoff modulation (octave-scaled), LFO→pitch (semitone-scaled), LFO→volume (tremolo)
- Filter envelope→cutoff modulation with configurable depth and independent ADSR

### Phase 8C: Drum Machine
- `DrumMachine`: 16-pad sample player implementing `InstrumentNode`, GM drum map (note 36+)
- `DrumPad`: one-shot/looped playback, fractional pitch shifting, decay envelope, equal-power pan law, velocity sensitivity
- `StepSequencer`: 16/32/64-step grid per pad, swing, per-step probability, accent, BPM-synced timing

### Phase 8D: Sampler
- `Sampler`: multi-sample instrument with key zones, velocity zones, 16-voice polyphony
- `SampleZone`: root key, key/velocity range mapping, loop modes (NoLoop, Forward, PingPong)
- Linear interpolation playback with pitch ratio calculation, voice stealing (oldest)
- 115 instrument tests total

### Phase 7A: AGNOS Agent API
- `AgentApi` — structured JSON API for AI agents to control sessions
  - Session: create, open, save, info
  - Tracks: add, list, gain, pan, mute, solo, add region
  - Transport: play, stop, pause, seek, set tempo
  - Export: bounce to WAV
  - Undo/redo
- `McpTools` — 5 MCP tool definitions matching daimon pattern
  - `shruti_session`, `shruti_tracks`, `shruti_transport`, `shruti_export`, `shruti_mixer`
  - Full dispatch routing with JSON schema validation

### Configurable Recording
- `RecordingConfig` struct: sample rate (44.1–192 kHz), channels (1–8), max duration, buffer size, input device selection
- Dynamic buffer sizing via `max_buffer_samples()` — adapts to configured rate/channels/duration
- `validated()` clamps all fields to safe ranges (snaps to nearest supported rate, clamps channels 1–8, buffer 64–4096)
- `Preferences.recording` field with `#[serde(default)]` for backward-compatible persistence
- `AudioEngine` methods: `set_recording_config()`, `recording_config()`, `recording_sample_rate()`, `recording_channels()`
- `start_recording()` uses config-driven format, device selection, and pre-allocated buffer (~10s headroom)
- UI record handler reads channels from engine config instead of hardcoded stereo
- 11 new RecordingConfig tests (defaults, validation, clamping, high-rate buffer calc, serialization roundtrip)

### Code Audit Fixes (Rounds 1–3: Security, Performance, Memory)
- **CRITICAL fix**: `Timeline::render()` — pre-allocated bus/source buffers (eliminated per-callback HashMap + Vec heap allocations in audio thread)
- **CRITICAL fix**: `SubtractiveSynth::process()` — replaced heap-allocated LFO Vecs with stack-allocated fixed-size arrays (zero allocations in audio path)
- **HIGH fix**: `Filter::process_sample()` — cached SVF coefficients (g, k, a1, a2, a3), tan() only recomputed when cutoff/resonance change
- **HIGH fix**: `analyze_spectrum()` — replaced panic-on-invalid-input with `Option<SpectralAnalysis>` return; added MAX_FFT_SIZE (65536) limit to prevent DoS via unbounded allocation
- **HIGH fix**: Agent API path traversal — `validate_path()` rejects `..` components in all file operations (open/save/export/add_region)
- **HIGH fix**: Export u64→u32 overflow — guard against session lengths exceeding u32::MAX before buffer allocation
- **HIGH fix**: Recording buffer — capped at 30 minutes (48kHz stereo) to prevent unbounded memory growth

### Code Audit Fixes (Rounds 6a–6b: Correctness, Concurrency)
- **HIGH fix**: Transport loop wrapping — `advance()` now uses modulo to handle multi-loop overshoot correctly
- **HIGH fix**: Sampler pitch ratio — guard against division by zero with `sample_rate.max(1.0)`
- **HIGH fix**: Atomics ordering — all `SharedTransport` operations upgraded from `Relaxed` to `Acquire`/`Release` pairs for correct cross-thread visibility
- **HIGH fix**: `update_session()` — meter_levels resize now happens inside session_data lock to prevent track count / meter slot mismatch
- **HIGH fix**: `points_in_range()` — boundary guard `end_idx.max(start_idx)` prevents panic when binary search returns inverted indices

### Code Review & Audit (Round 7)
- **CRITICAL fix**: `FilePlayerNode` — empty buffer with looping no longer panics (early-return silence)
- **CRITICAL fix**: Reverb comb/allpass filters — buffer size clamped to min 1 (prevents modulo-by-zero panic at very low sample rates)
- **CRITICAL fix**: Spectral analysis Hann window — handles n=1 without division by zero
- **CRITICAL fix**: `MoveTrack` undo — bounds check on `from_index` before `remove()` (prevents panic on invalid command)
- **HIGH fix**: SVF filter — resonance clamped to [0,1] and cutoff clamped to [20, 0.49×sr] in coefficient computation (prevents NaN/infinity)
- 723 tests, 59.4% line coverage (excluding vendor code)

### Test Coverage Push
- `shruti-dsp`: 67→168 tests (+101) — error types, reverb, compressor, delay, limiter, buffer, meter, format
- `shruti-engine`: 9→55 tests (+46) — FilePlayerNode (empty buffer, looping, reset), GainNode, NodeId, graph compilation, topological sort, record manager
- `shruti-instruments`: 115→120 tests (+5) — filter set_sample_rate, dynamic cutoff, resonance clamping, nyquist clamping, stress test
- `shruti-plugin`: 3→19 tests (+16) — PluginHost lifecycle, load error paths (CLAP/VST3/Native), find_vst3_binary, scanner
- `shruti-session`: 131→137 tests (+6) — SessionError Display/source/From impls, RecordingConfig
- Added `tarpaulin.toml` to exclude vendor code from coverage metrics

### Instrument Testing
- Envelope timing accuracy tests: attack, decay, release within ±1ms at 44100/48000/96000 Hz
- Sample rate change test: verifies timing consistency across different rates
- Preset roundtrip tests: SubtractiveSynth (with audio output verification), DrumMachine, Sampler
- Cross-instrument JSON preset roundtrip test for all 3 instrument types
- 12 new tests total (8 envelope timing + 4 preset roundtrip)

### Parameter Automation Exposure
- `AutomationTarget::InstrumentParam { param_index }` — all instrument parameters now automatable
- `AutomationTarget::label()` — human-readable display label for any target
- `AutomationTarget::instrument_targets(param_count)` — generates full target list for an instrument track
- 4 tests: instrument param automation, target list generation, labels, serde roundtrip

### Track Templates
- `TrackTemplate`: reusable track configuration (kind, gain, pan, channels, instrument params, color)
- `from_track()` captures settings without content; `create_track()` instantiates with new ID
- File save/load as JSON (`save()` / `load()`)
- 5 tests: capture settings, create track, serde roundtrip, file I/O, error handling

### Track Kind Icons & Colors
- `TrackKind::icon()` — distinct Unicode icon per kind (Audio, Bus, MIDI, Master, Instrument)
- `TrackKind::default_color()` — unique RGB default color per kind (blue, amber, green, red, purple)
- `TrackKind::label()` — short text label per kind
- `Track::color` field with `#[serde(default)]` for user-customizable color override
- `Track::display_color()` — returns custom color or kind default
- 6 tests: distinct icons, distinct colors, labels, default color, override color, backward compat serde

### Sample Format Support
- AIFF and OGG/Vorbis import via symphonia feature flags (`aiff`, `ogg`, `vorbis`)
- `SUPPORTED_EXTENSIONS` constant and `is_supported_extension()` helper
- `read_audio_file()` now supports WAV, FLAC, AIFF, OGG
- 4 reader tests: extension validation, WAV roundtrip, nonexistent file, invalid data

### Per-Pad Effects
- `PadEffects` per drum pad: one-pole LPF (cutoff 0–1), tanh drive saturation, reverb/delay send levels
- Integrated into `DrumPad::tick()` — filter and drive applied post-pan
- Saved/loaded in `DrumKitPad` with `#[serde(default)]` for backward compatibility
- 6 tests: default values, passthrough, filter attenuation, drive saturation, pad integration, reset

### Kit Management
- `DrumKit` preset: captures all 16 pad configurations (name, pitch, gain, pan, decay, play_mode, midi_note) as JSON
- `DrumKitPad` with `from_pad()` / `apply_to()` for individual pad snapshot/restore
- `DrumKit::from_drum_machine()` / `apply_to()` for full kit capture and restore
- Optional `sample_path` per pad for sample file references on reload
- File save/load (`save()` / `load()`) with JSON serialization
- 8 tests: capture, restore, partial kit, serde roundtrip, file I/O, error handling, sample paths

### Per-Instrument Effects
- `EffectChain` with scratch buffer and `process_with()` closure-based API for borrow-safe processing
- 5 effect types: Chorus (modulated delay line), Delay (reuses `shruti_dsp`), Reverb (reuses `shruti_dsp`), Distortion (tanh soft clipping), FilterDrive (tanh saturation + one-pole LPF)
- `InstrumentEffect` with enable/disable, dry/wet mix, per-type internal state
- Integrated into all 3 instruments: `SubtractiveSynth`, `DrumMachine`, `Sampler` via `std::mem::take` pattern
- 13 effect chain tests: creation, add/remove, passthrough, disabled effects, each effect type, dry/wet mix, sample rate changes, reset

### Track Grouping
- `TrackGroup` with `TrackGroupId`, name, ordered member list, collapsible state
- `Session` group methods: `add_group()`, `remove_group()`, `add_track_to_group()`, `remove_track_from_group()`, `rename_group()`, `toggle_group_collapsed()`, `track_group()` lookup
- Master track cannot be added to groups; removing a track auto-cleans group membership
- 6 new `EditCommand` variants with full undo/redo: `CreateGroup`, `RemoveGroup`, `AddTrackToGroup`, `RemoveTrackFromGroup`, `RenameGroup`, `ToggleGroupCollapsed`
- Arrangement view: collapsible group headers with arrow indicator and member count
- Mixer view: group divider strips with collapse toggle
- Serializable with `#[serde(default)]` for backward-compatible session loading
- 19 new tests: TrackGroup CRUD, serde roundtrip, session group operations, undo/redo for all 6 group commands

### Sample Layering (8C.5)
- `SampleLayer` struct with velocity range matching and per-layer sample data
- `LayerSelection` enum: RoundRobin (deterministic cycling) or Random
- `DrumPad` layer support: `add_layer()`, `remove_layer()`, `clear_layers()`
- Velocity-based layer selection with fallback to main samples when no layer matches
- Active layer samples used in `tick()` for seamless layer/main switching
- 7 tests: velocity selection, fallback, round-robin cycling, random selection, add/remove/clear, matches

### Sample Editing (8D.2)
- `SampleZone` editing methods: `trim()`, `set_loop_points()`, `clear_loop_points()`, `fade_in()`, `fade_out()`, `normalize()`, `reverse()`
- Analysis: `peak()`, `rms()`, `len()`, `is_empty()` for sample inspection
- 15 tests covering all editing operations

### Output Routing Matrix (8F.8)
- `OutputRouting` struct with `output: Option<TrackId>` and `sidechain_input: Option<TrackId>`
- `Session::set_track_output()`, `set_sidechain_input()`, `track_output_chain()`
- `would_create_routing_loop()` — prevents routing loops via chain walking
- `EditCommand::SetTrackOutput` and `SetSidechainInput` with full undo/redo
- Routing tests: basic routing, loop detection, sidechain assignment, chain walking

### Instrument UI (8E)
- **Instrument rack panel**: `instrument_panel_view()` — detects instrument track kind, renders track header with kind-specific info, generic parameter knob grid (4 per row), preset placeholder
- **Synth editor**: `synth_editor_view()` — collapsible sections for Oscillators (3 osc with enable/waveform/detune/level, hard sync, ring mod, FM), Amplitude Envelope (ADSR + volume), Filter (cutoff/resonance/mode selector), Filter Envelope (ADSR + depth), dual LFOs (rate/depth/target/shape)
- **Drum machine grid**: `drum_grid_view()` — 4×4 pad grid with GM drum names, selected pad controls (pitch/gain/pan/decay knobs), 16-step sequencer with toggle buttons, pattern bank selector (A/B/C/D), swing knob
- **Sampler editor**: `sampler_editor_view()` — keyboard zone map (128-key visualization with colored zone rectangles), zone controls (root key, key/vel range sliders, gain, loop mode), waveform drop zone placeholder, global controls
- **Piano roll**: `piano_roll_view()` — 128-note scrollable grid with piano key labels, instrument-aware display (GM drum names for DrumMachine tracks, key range highlighting for Instrument/Sampler), velocity-based note opacity, beat/bar grid
- `ViewMode::InstrumentEditor` and `ViewMode::PianoRoll` added to view switcher
- 71 new UI tests (19 instrument panel, 16 synth editor, 9 drum grid, 14 sampler editor, 13 piano roll)

### Track Types (8F.2, 8F.3, 8F.4)
- `TrackKind::DrumMachine { kit_name, pad_count }` — drum icon, orange default color, `Session::add_drum_machine_track()`
- `TrackKind::Sampler { preset_name, zone_count }` — disc icon, teal default color, `Session::add_sampler_track()`
- `TrackKind::AiPlayer { model_name, style, creativity }` — robot icon, deep purple default color, `Session::add_ai_player_track()`
- Manual `PartialEq` impl for `TrackKind` to handle `f32` creativity field (bitwise equality)
- All variants backward-compatible via `#[serde(default)]`
- 28 new tests (9 per track type + updated distinctness tests)

### Multi-Oscillator Expansion (8B+.6)
- 3 oscillators per voice with independent waveform, detune (cents), and level
- Osc2/Osc3 enable toggles with backward-compatible defaults (disabled)
- Hard sync: osc1 resets osc2 phase on zero crossing for classic sync timbres
- Ring modulation: blendable osc1 × osc2 product (0–1 mix)
- FM synthesis: osc1→osc2 frequency modulation with configurable depth
- Level normalization: divides by active oscillator count to prevent clipping
- 11 new parameters (PARAM_OSC2_ENABLE through PARAM_FM_AMOUNT)
- 16 multi-oscillator tests

### SFZ/SF2 Import (8D.6)
- SFZ text format parser: `<global>`, `<group>`, `<region>` blocks with opcode inheritance
- SFZ opcodes: sample, lokey/hikey/key, lovel/hivel, pitch_keycenter, loop_mode/start/end, tune, volume, pan
- Note name parsing (e.g., `c4`, `f#3`), comment stripping, multi-header lines
- SF2 binary RIFF parser: full sfbk format (phdr, pbag, pgen, inst, ibag, igen, shdr chunks)
- Preset→instrument→sample zone resolution with key/velocity range merging
- 16-bit PCM extraction from sdta/smpl chunk, loop mode support, ROM sample skipping
- No external dependencies — both parsers implemented from scratch
- 28 tests (16 SFZ + 12 SF2)

### Pattern System (8C.3)
- `PatternBank` enum (A/B/C/D), `Pattern` struct with 64 total patterns (4 banks × 16)
- `PatternChain` for song mode pattern sequencing
- `StepSequencer` extended: `select_pattern()`, `copy_pattern()`, `set_chain()`, `next_pattern_in_chain()`
- 16 pattern tests + 20 timing tests

### Slice Mode (8D.4)
- `SlicePoint` struct with position, name, and zone reference
- Energy-based onset detection: 1024-sample window, 512-sample hop, minimum 2048-sample gap
- `auto_slice_by_transients()` for automatic sample slicing
- `slice_to_zones()` maps slices to MIDI keys (REX-style)
- 7 slice tests

### Modulation Matrix (8B.2)
- `ModSource` enum (8 sources: LFO1/2, AmpEnv, FilterEnv, Velocity, Aftertouch, ModWheel, PitchBend)
- `ModDestination` enum (8 destinations: Pitch, Cutoff, Resonance, Volume, Pan, FilterEnvDepth, LfoRate, LfoDepth)
- `ModRouting` with source, destination, bipolar amount (-1..1), enable toggle
- `ModMatrix` with max 16 routings and `evaluate()` method
- `ModSourceValues` input and `ModOutput` result structs
- 14 modulation matrix tests

### HTTP Server (16A)
- `shruti serve --port 8050` CLI subcommand for AGNOS integration
- axum HTTP server wrapping `AgentApi` with shared `Arc<Mutex>` state
- 8 endpoints: `/health`, `/api/session`, `/api/tracks`, `/api/transport`, `/api/export`, `/api/mixer`, `/api/analysis`, `/api/mcp`
- Permissive CORS layer for cross-origin agent access
- 16 async tests

### Instrument Testing (8G)
- Oscillator accuracy tests: frequency accuracy within ±1Hz across waveforms, DC offset checks, amplitude consistency
- Filter response tests: cutoff accuracy, resonance boost, mode switching (LP/HP/BP/Notch), slope verification
- Polyphony stress tests: max voice allocation, voice stealing modes (oldest/quietest/lowest), under load
- Step sequencer tests: timing accuracy, swing calculation, probability distribution, BPM sync
- Sample playback tests: pitch mapping, loop points, velocity layers, one-shot vs gated, drum machine playback
- MIDI integration tests: end-to-end MIDI clip → instrument → audio for synth, drum machine, sampler
- 1292 total tests across workspace

### Engineering Backlog
- **Engine**: Lock-free meter levels (`AtomicStereoLevel`), mono→stereo upmix, poisoned mutex recovery with logging, render failure logging, diff-based device cache refresh
- **DSP**: EBU R128 compliant LUFS (per-channel mean-square), standard compressor soft knee, `catch_unwind` for audio parsing, reverb/allpass min buffer size, delay clamp
- **Instruments**: Envelope stage_pos reset on all transitions, LFO S&H double-sample fix, `fast_exp2()` for pitch modulation, ADSR helper deduplication (`read_adsr()`), sample rate observer, filter docs
- **Session**: Audio pool LRU eviction, sorted regions with binary search, `VecDeque` for undo stack, schema validation on load, sorted MIDI events, automation dead code cleanup
- **UI Critical**: Auto-save with `.shruti_backup` (60s interval), background file I/O via `mpsc`, save prompt on New/Open, toast notifications, comprehensive undo in mixer (mute/solo/gain/pan), engine init error dialog
- **UI Medium**: Waveform peaks caching, snap-to-grid, recording animation, grid level-of-detail, zoom boundary clamping (`clamp_zoom`/`zoom_to_fit`), keyboard shortcuts, audio pool persistence, theme caching
- **Security**: MCP request size limit, agent API rate limiting, plugin scanner symlink depth, preferences file permissions (0600), plugin state blob validation, scanner disk cache
### Code Audit Round 8 (Memory, Security, Performance, Quality)
- **Security**: HTTP server bound to localhost only (was 0.0.0.0), CORS restricted to localhost origins, `create_session` validates sample_rate/buffer_size ranges, error responses sanitized (no internal path leakage), SF2 parser `saturating_sub` for malformed loop points
- **Memory**: Pre-allocated `node_outputs` HashMap in GraphProcessor (eliminates per-callback heap allocation), pre-allocated `dry_buffer` in InstrumentEffect, iterator-based `select_layer()` (zero allocation), LUFS blocks capped at 1500 entries
- **Performance**: `#[inline]` on all per-sample DSP functions (AudioBuffer::get/set, Oscillator::sample/advance_phase, Envelope::tick, Lfo::tick, CombFilter/AllpassFilter::process), `fast_exp2_f64()` minimax polynomial for oscillator detune, pre-computed osc2/osc3 detune ratios outside per-sample loop, `fast_linear_to_db()`/`fast_db_to_linear()` IEEE 754 bit-tricks in compressor, cached pan gains in DrumPad
- **Quality**: All clippy warnings fixed (31 across 12 files), dead code eliminated, silent error swallowing replaced with `eprintln!` logging, reverb safety tests for low sample rates
- **E2E Integration Tests**: 23 cross-crate tests — full audio pipeline, instrument pipeline (synth + drum machine), effects chain (EQ→compressor→reverb→delay), session persistence roundtrip, comprehensive undo/redo (12 ops), audio format roundtrip (WAV int16/int24/float32), preset roundtrip (all 3 instruments), MIDI routing (velocity curves, channel filter, note range), sampler zone mapping, bus send rendering, automation timeline render
- 1327 total tests across workspace, 0 clippy warnings, 0 audit vulnerabilities

### CI/CD
- GitHub Actions: CI (fmt, clippy, audit, test, build)
- GitHub Actions: Release (Linux amd64/arm64, macOS x86/arm, Windows)
- CalVer versioning (`YYYY.M.D` or `YYYY.M.D-N`) with `bump-version.sh`
