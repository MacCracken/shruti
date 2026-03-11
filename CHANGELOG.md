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

### CI/CD
- GitHub Actions: CI (fmt, clippy, audit, test, build)
- GitHub Actions: Release (Linux amd64/arm64, macOS x86/arm, Windows)
- CalVer versioning (YYYY.M.D-N) with `bump-version.sh`
