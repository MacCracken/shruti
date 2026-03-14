# ADR-003: Type-Safe Instrument Parameter Enums

**Status:** Accepted
**Date:** 2026-03-14

## Context

Instrument parameters were identified by magic number constants (`const PARAM_WAVEFORM: usize = 0`, `const PARAM_ATTACK: usize = 1`, etc.). This pattern is error-prone: nothing prevents passing an out-of-range index, using the wrong instrument's constant, or confusing parameter indices with other `usize` values.

## Decision

Replace magic constants with `#[repr(usize)]` enums per instrument:

- `SynthParam` — 34 variants (Waveform through FmAmount)
- `SamplerParam` — 5 variants (Volume, Attack, Decay, Sustain, Release)
- `DrumMachineParam` — 1 variant (Volume)

A `ParamIndex` trait provides:
- `index(self) -> usize` — convert enum to array index
- `count() -> usize` — total parameter count for the instrument

Each enum also implements `From<Enum> for usize`, `TryFrom<usize> for Enum`, and typed `get_param()`/`set_param()` methods on the instrument structs.

## Consequences

- **Compile-time parameter validation** — impossible to pass an invalid index when using the enum API.
- **`TryFrom` for runtime validation** — UI code (which uses `usize` indices from `track.instrument_params`) can validate at the boundary.
- **No backward compat issue** — the UI crate maintains its own `usize` constants for indexing into `track.instrument_params: Vec<f32>`. These will be migrated to use the enum re-exports in a future pass.
- **Self-documenting** — `SynthParam::FilterCutoff.index()` is clearer than `PARAM_FILTER_CUTOFF`.
