# Shruti Roadmap — Path to MVP v1

## Vision

Shruti MVP v1 is a functional DAW capable of recording, editing, mixing, and exporting audio with plugin support. It should be usable for real music production, not just a tech demo.

---

## Phase 1: Foundation

**Goal:** Audio plays through the system reliably.

- [ ] Workspace setup — Cargo workspace with crate structure
- [ ] Platform audio backends — ALSA, PipeWire, CoreAudio, WASAPI abstraction
- [ ] Audio graph — Lock-free, real-time safe node graph
- [ ] Basic buffer management — Sample-accurate playback and recording
- [ ] Audio file I/O — WAV and FLAC read/write
- [ ] CLI playback tool — `shruti-play` for testing the engine headless

**Exit criteria:** Can play back and record a WAV file on all three platforms with <10ms latency.

---

## Phase 2: Session & Tracks

**Goal:** Multi-track timeline with non-destructive editing.

- [ ] Session/project model — Serializable project format (likely SQLite + sidecar audio)
- [ ] Track types — Audio tracks, bus tracks, master bus
- [ ] Timeline — Region-based, non-destructive clip arrangement
- [ ] Basic editing — Cut, copy, paste, move, trim, fade in/out
- [ ] Transport — Play, pause, stop, loop, seek, tempo/time signature
- [ ] Undo/redo — Command-pattern undo with full history

**Exit criteria:** Can arrange a multi-track session, edit regions, and play it back seamlessly.

---

## Phase 3: Mixing

**Goal:** Professional signal routing and built-in effects.

- [ ] Mixer — Per-track gain, pan, mute, solo
- [ ] Sends & returns — Aux buses with pre/post-fader sends
- [ ] Built-in DSP — EQ (parametric), compressor, reverb, delay, limiter
- [ ] Metering — Peak, RMS, LUFS metering on all channels
- [ ] Automation — Parameter automation with lanes and curves

**Exit criteria:** Can produce a mixed-down track with EQ, compression, reverb, and automation.

---

## Phase 4: Plugin Hosting

**Goal:** Load and use third-party audio plugins.

- [ ] CLAP host — Full CLAP plugin hosting support
- [ ] VST3 host — VST3 plugin scanning, loading, and parameter control
- [ ] Plugin UI — Embed plugin GUIs or provide generic parameter UI
- [ ] Plugin state — Save/restore plugin state with sessions
- [ ] Sandboxing — Process-isolated plugin hosting for crash safety

**Exit criteria:** Can load popular VST3/CLAP synths and effects, automate their parameters, and save/recall state.

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

## Phase 7: AGNOS Integration (Post-MVP, pre-v1 tag)

**Goal:** First-class AI agent support on AGNOS.

- [ ] Agent API — Structured command interface for session control
- [ ] Agent mixing — AI-driven mix suggestions and automation
- [ ] Voice control — Natural language session commands via AGNOS agents
- [ ] Analysis — Real-time spectral and dynamics analysis exposed to agents

**Exit criteria:** An AGNOS agent can open a session, arrange tracks, apply effects, mix, and export — with human oversight.

---

## MVP v1 Release

Phases 1–6 complete. Phase 7 follows as the first post-MVP milestone to align with the AGNOS ecosystem.
