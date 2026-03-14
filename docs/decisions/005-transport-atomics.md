# ADR-005: Atomic Transport Synchronization

**Status:** Accepted
**Date:** 2026-03-14

## Context

The UI thread and audio thread both need access to transport state (position, play/pause, recording, loop settings). The audio thread writes position on every callback (~200 times/second); the UI reads position for playhead display and writes control signals (play, stop, seek).

Direct shared writes to `position` from both threads would race.

## Decision

Use `SharedTransport` with per-field atomics and a **seek request slot**:

- `position: AtomicU64` — written by audio thread (advance), read by UI (playhead display). Only the audio thread writes this field during playback.
- `seek_request: AtomicU64` — written by UI, consumed by audio thread via `swap(NO_SEEK)`. The sentinel `u64::MAX` means "no seek pending". This avoids a write race on `position`.
- `playing`, `recording`: `AtomicBool` — written by UI, read by audio thread.
- `loop_enabled`, `loop_start`, `loop_end`: atomics synced via `sync_loop()` from UI transport state.

All loads use `Ordering::Acquire`, all stores use `Ordering::Release`.

## Consequences

- **No write races** — position has a single writer (audio thread). Seek is a request, not a direct write.
- **Loop handling in audio callback** — the audio thread reads loop bounds atomically and handles wrap-around with modulo, matching the session `Transport::advance()` logic.
- **UI position is slightly behind** — the UI reads position that was written at the end of the previous audio callback. At 48kHz/256 frames this is ~5ms of latency, imperceptible for playhead display.
