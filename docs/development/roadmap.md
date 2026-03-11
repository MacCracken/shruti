# Shruti Roadmap — Path to MVP v1

> **Version**: 2026.3.11 | **Last Updated**: 2026-03-11
> **Status**: Phases 1–6, 7A, 7B complete — MVP v1 reached
> **Tests**: 195 passing (31 dsp, 6 engine, 30 session, 3 plugin, 10 ai, 115 ui), 40% line coverage, 0 clippy warnings, 0 audit vulnerabilities

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
| 7A — Agent API | AI agent control | AgentApi (session/tracks/transport/export), 5 MCP tools, daimon integration |
| 7B — Agnoshi | Natural language | 7 intent patterns, translate module, curl bridge |

---

## Phase 7: AGNOS Integration (remaining)

**Goal:** First-class AI agent support on AGNOS.

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

## Crate Architecture

| Crate | Purpose | Status |
|-------|---------|--------|
| `shruti-engine` | Real-time audio engine, cpal backend, lock-free graph | Active |
| `shruti-dsp` | Audio buffers, format types, file I/O, effects, metering | Active |
| `shruti-session` | Session, tracks, regions, timeline, transport, undo, MIDI, preferences | Active |
| `shruti-plugin` | Plugin hosting: CLAP, VST3, native Rust | Active |
| `shruti-ui` | GPU-accelerated DAW UI (egui + eframe) | Active |
| `shruti-ai` | Agent API + MCP tools for AGNOS | Active |

---

## MVP v1 Release

Phases 1–6 complete. Phase 7 and MIDI 2.0 follow as post-MVP milestones.

---

## Test Coverage Roadmap (40% → 80%)

Current: 195 tests, 40% line coverage (1200/2997 lines).

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

*Last Updated: 2026-03-11*
