# ADR-002: Domain Newtypes for Type Safety

**Status:** Accepted
**Date:** 2026-03-14

## Context

The session model uses `u64` for frame positions, durations, offsets, and fades, and `usize` for track slot indices. These are easily confused with other numeric values (buffer sizes, sample counts, array indices), leading to potential bugs that the compiler cannot catch.

## Decision

Introduce transparent newtypes:

- **`FramePos(pub u64)`** — for all frame-based positions and durations on the timeline.
- **`TrackSlot(pub usize)`** — for track slot indices into the session's track list.

Both use `#[serde(transparent)]` for backward-compatible JSON serialization and implement arithmetic ops (`Add`, `Sub`, `Rem`, `AddAssign`, `SubAssign`), comparison traits, and conversion traits (`From<u64>`, `From<u32>`).

Raw access via `.0` is available for interop with APIs that require primitive types (atomic operations, buffer indexing).

## Consequences

- **Compile-time safety** — passing a buffer size where a frame position is expected is now a type error.
- **Backward-compatible serialization** — existing session files load without migration.
- **Verbose construction** — test code uses `FramePos(100)` instead of `100`. Constructors accept `impl Into<FramePos>` to ease this.
- **Wide migration** — touched all 7 crates and ~50 files. Future changes in the same domain are safer.
