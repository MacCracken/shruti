# ADR-001: Double-Buffered Real-Time Data Sharing

**Status:** Accepted
**Date:** 2026-03-14

## Context

The audio callback runs on a real-time thread and must never block. Session data (tracks, audio pool) and the graph execution plan need to be shared between the UI thread and the audio thread.

The original design used `Arc<Mutex<T>>` with `try_lock` on the RT thread. On contention, the audio callback would output silence — causing audible dropouts during UI operations like saving or editing tracks.

## Decision

Use a **double-buffered pending slot** pattern for all RT-shared data:

1. The RT thread owns a local copy of the data (no lock to read).
2. The non-RT thread places updates into a `Mutex<Option<T>>` pending slot.
3. Each callback, the RT thread calls `try_lock` on the pending slot:
   - **Success + Some:** Swap new data into the local copy.
   - **WouldBlock:** Continue with the existing local copy (no silence).
   - **Poisoned:** Recover via `into_inner()` and attempt to pick up the pending data.

This pattern is applied to both `SharedSessionData` and `GraphProcessor`'s execution plan.

## Consequences

- **No silence on contention** — the RT thread always has valid data to render.
- **Slightly stale data** — during a UI update, the RT thread may render one buffer cycle with the previous track/pool state. This is imperceptible (<6ms at 256 frames/48kHz).
- **Memory overhead** — two copies of session data exist simultaneously (the local copy and the pending slot). Acceptable for track metadata; audio pool data is `Arc`-shared.
- **Poisoned mutex recovery** — the RT thread never panics on a poisoned mutex. It logs and recovers.
