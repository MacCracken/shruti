# Changelog

All notable changes to Shruti are documented here.
Format: CalVer (YYYY.M.D-N).

## 2026.3.11-0 — Initial Release

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

### CI/CD
- GitHub Actions: CI (fmt, clippy, audit, test, build)
- GitHub Actions: Release (Linux amd64/arm64, macOS x86/arm, Windows)
- CalVer versioning (YYYY.M.D-N) with `bump-version.sh`
