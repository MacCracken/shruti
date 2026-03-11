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

### Phase 4: Plugin Hosting
- `PluginInstance` trait — unified API for all plugin formats
- CLAP host — load .clap plugins, verify `clap_entry` symbol
- VST3 host — load .vst3 bundles, platform-aware binary discovery
- Native Rust plugin API — `shruti_plugin_create` entry point
- `PluginScanner` — scan standard paths on Linux/macOS/Windows
- `PluginState` — serializable parameter values + opaque binary chunk
- `PluginNode` — integrate any plugin into the audio graph
- `PluginHost` — manage plugin instances with slot-based activation

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
