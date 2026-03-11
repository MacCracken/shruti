# Shruti Roadmap — Path to MVP v1

> **Version**: 2026.3.11 | **Last Updated**: 2026-03-11
> **Status**: Phases 1, 2, 3, 4, 7A, 7B complete — Phase 5 next
> **Tests**: 70 passing (29 dsp, 6 engine, 23 session, 3 plugin, 9 ai), 0 clippy warnings, 0 audit vulnerabilities

## Vision

Shruti MVP v1 is a functional DAW capable of recording, editing, mixing, and exporting audio with plugin support. It should be usable for real music production, not just a tech demo. Purpose-built as the primary audio workstation for the AGNOS ecosystem.

---

## Phase 1: Foundation (Complete)

**Goal:** Audio plays through the system reliably.

- [x] Workspace setup — Cargo workspace with 6 crates (engine, dsp, plugin, ui, session, ai)
- [x] Platform audio backends — ALSA, PipeWire, JACK, CoreAudio, WASAPI via cpal abstraction
- [x] Audio graph — Lock-free node graph with topological execution plan
- [x] Basic buffer management — `AudioBuffer` with interleaved storage, per-channel access
- [x] Audio file I/O — WAV/FLAC/ADPCM read (symphonia), WAV write (hound)
- [x] CLI playback tool — `shruti-play` headless playback binary

**Exit criteria:** Can play back and record a WAV file on all three platforms with <10ms latency.

---

## Phase 2: Session & Tracks (Complete)

**Goal:** Multi-track timeline with non-destructive editing.

- [x] Session/project model — `Session` with SQLite persistence + sidecar audio pool
- [x] Track types — Audio tracks, bus tracks, master bus (`TrackKind` enum)
- [x] Timeline — Region-based, non-destructive clip arrangement with fade in/out
- [x] Basic editing — Split, move, trim, fade via `EditCommand` enum + undo integration
- [x] Transport — Play, pause, stop, loop, seek, tempo/time signature, BPM↔frame conversions
- [x] Undo/redo — Command-pattern undo/redo with full history (`UndoManager`, 1000 deep)

**Exit criteria:** Can arrange a multi-track session, edit regions, and play it back seamlessly.

---

## Phase 3: Mixing (Complete)

**Goal:** Professional signal routing and built-in effects.

- [x] Mixer — Per-track gain, pan, mute, solo with stereo balance panning in timeline
- [x] Sends & returns — `Send` struct with pre/post-fader routing to bus tracks
- [x] Built-in DSP — `ParametricEq` (biquad, multi-band), `Compressor`, `Reverb` (Freeverb), `Delay` (stereo), `Limiter` (brickwall), `StereoPanner`
- [x] Metering — `Meter` with peak, RMS, integrated LUFS (EBU R128 gating)
- [x] Automation — `AutomationLane` with points, Linear/Step/SCurve interpolation, per-frame value lookup

**Exit criteria:** Can produce a mixed-down track with EQ, compression, reverb, and automation.

---

## Phase 4: Plugin Hosting (Complete)

**Goal:** Load and use third-party audio plugins.

- [x] Plugin abstraction — `PluginInstance` trait with unified API across formats
- [x] CLAP host — Load CLAP plugins via `clap_entry`, parameter control
- [x] VST3 host — Load VST3 bundles via `GetPluginFactory`, platform-aware binary discovery
- [x] Native Rust plugins — Shruti-native plugin API via `shruti_plugin_create`
- [x] Plugin scanner — Scan standard paths on Linux/macOS/Windows for all formats
- [x] Plugin state — Serializable `PluginState` with params + opaque chunk data
- [x] Plugin graph node — `PluginNode` integrates any plugin into the audio graph
- [ ] Plugin UI — Embed plugin GUIs (deferred to Phase 5)
- [ ] Sandboxing — Process-isolated hosting (deferred to Phase 6)

**Exit criteria:** Can scan, load, and process audio through CLAP/VST3/Native plugins with parameter control and state save/restore.

---

## Phase 5: UI

**Goal:** GPU-accelerated interface for the full DAW workflow.

- [ ] Rendering backend — wgpu-based 2D rendering
- [ ] Arrangement view — Timeline with tracks, clips, and waveforms
- [ ] Mixer view — Channel strips, faders, meters, plugin slots
- [ ] Transport bar — Playback controls, tempo, time display
- [ ] Browser — File browser and plugin browser panels
- [ ] Keyboard shortcuts — Configurable key bindings
- [ ] Theming — Dark theme with customization support

**Exit criteria:** Full DAW workflow achievable through the GUI without CLI fallback.

---

## Phase 6: Export & Polish

**Goal:** Production-ready output and workflow refinements.

- [ ] Export — Bounce to WAV, FLAC, MP3, OGG with format options
- [ ] MIDI — Basic MIDI track support (record, edit, route to plugins)
- [ ] Drag and drop — File import via drag and drop
- [ ] Preferences — Audio device selection, buffer size, sample rate config
- [ ] Error handling — Graceful recovery from plugin crashes, xruns
- [ ] Documentation — User guide, keyboard shortcut reference

**Exit criteria:** Can produce and export a finished track. Ready for real-world use.

---

## Phase 7: AGNOS Integration

**Goal:** First-class AI agent support on AGNOS. Shruti becomes a native AGNOS
application with agent-driven music production, MCP tool access, and deep
integration with daimon, hoosh, and agnoshi.

### 7A — Agent API & MCP Tools (Complete)

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1 | Session control API | Done | `AgentApi`: create, open, save, info |
| 2 | Track & region manipulation API | Done | add track, add region, gain/pan/mute/solo |
| 3 | Mixer control API | Done | list tracks, undo/redo |
| 4 | Export API | Done | `export_wav()` — bounce session to WAV |
| 5 | MCP tools (5): `shruti_*` | Done | `McpTools::tool_manifest()` + `dispatch()` |
| 6 | Register in daimon MCP server | Done | Wired into agnosticos `mcp_server.rs` — 5 tools + dispatch + handlers (30 tests) |

### 7B — Agnoshi Integration (Complete)

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1 | Agnoshi intent patterns (5) | Done | 7 patterns in `patterns.rs`, parse logic in `parse.rs` (22 tests) |
| 2 | Translate module | Done | `translate/shruti.rs` calling MCP tools via curl bridge |
| 3 | Natural language session commands | Medium | "record vocals on track 2", "add reverb to guitar" |

### 7C — AI-Assisted Production

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Auto-mix agent | Large | AI-driven gain staging, EQ, compression suggestions |
| 2 | Spectral analysis API | Medium | Real-time FFT exposed to agents for frequency analysis |
| 3 | Dynamics analysis API | Medium | RMS, peak, crest factor, loudness (LUFS) for agents |
| 4 | Composition suggestions | Large | Agent proposes arrangement changes, chord progressions |
| 5 | Voice control via vansh | Medium | "play from bar 16", "mute the drums", "louder on vocals" |

### 7D — AGNOS Distribution

| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1 | Takumi recipe (`recipes/marketplace/shruti.toml`) | Done | Build from source, native binary, desktop entry, systemd service |
| 2 | Marketplace recipe with `github_release` | Done | Auto-version from release tags |
| 3 | Sandbox profile | Done | Audio device access (PipeWire/ALSA), Landlock, Wayland protocols |
| 4 | Argonaut service integration | Small | Optional auto-start in Desktop mode |
| 5 | Aethersafha Wayland integration | Medium | Embed in compositor, proper surface management |

**Exit criteria:** An AGNOS agent can open a session, arrange tracks, apply effects, mix, and export — with human oversight. Shruti installable from mela marketplace.

---

## Crate Architecture

| Crate | Purpose | Status |
|-------|---------|--------|
| `shruti-engine` | Real-time audio engine, cpal backend, lock-free graph | Active |
| `shruti-dsp` | Audio buffers, format types, file I/O, effects, metering | Active |
| `shruti-session` | Session, tracks, regions, timeline, transport, undo | Active |
| `shruti-plugin` | Plugin hosting: CLAP, VST3, native Rust | Active |
| `shruti-ui` | wgpu-based GPU UI | Stub |
| `shruti-ai` | Agent API + MCP tools for AGNOS | Active |

---

## MVP v1 Release

Phases 1–6 complete. Phase 7 follows as the first post-MVP milestone to align with the AGNOS ecosystem.

*Last Updated: 2026-03-11*
