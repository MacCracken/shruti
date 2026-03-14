# ADR-004: Per-Crate Error Types

**Status:** Accepted
**Date:** 2026-03-14

## Context

Error handling was inconsistent across the workspace:
- `shruti-session` had `SessionError` (well-designed, with `From` impls)
- `shruti-dsp` had `AudioError` (well-designed)
- `shruti-engine` used `Box<dyn std::error::Error>` — lost type information
- `shruti-plugin` used `Result<_, String>` — lost error structure and source chains
- `shruti-instruments` used `Result<_, String>` for SF2/SFZ parsing

## Decision

Add typed error enums to every crate that previously used generic errors:

- `EngineError` — Backend, Graph, Recording, Io
- `PluginError` — NotFound, LoadError, StateError, ScanError, Io
- `InstrumentError` — ParseError, InvalidConfig, Io

Each implements:
- `std::fmt::Display` — human-readable messages
- `std::error::Error` — with `source()` for Io variants
- `From<std::io::Error>` — automatic I/O error wrapping
- Crate-specific `From` impls (e.g., `From<String>` for backward compat)

Leaf crates (`shruti-ui`, `shruti-ai`) continue to use `Box<dyn Error>` at their public API boundaries since they are application-level code, not libraries.

## Consequences

- **Pattern matching on errors** — callers can distinguish "plugin not found" from "plugin load failed" without parsing strings.
- **Error source chains** — `Io` variants preserve the underlying `std::io::Error` for debugging.
- **No cross-crate super-error** — we chose per-crate enums over a single `ShrutiError` to keep crate boundaries clean. A top-level error can be added later if needed.
