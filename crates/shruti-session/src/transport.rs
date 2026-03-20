use serde::{Deserialize, Serialize};

use crate::types::FramePos;

/// Transport playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
    Recording,
}

/// Result of advancing the transport, with loop boundary information.
#[derive(Debug, Clone, Copy)]
pub struct AdvanceResult {
    /// Position before the advance.
    pub start: FramePos,
    /// Position after the advance.
    pub end: FramePos,
    /// Whether the transport wrapped past the loop boundary.
    pub loop_wrapped: bool,
    /// Current loop iteration (0-indexed, increments on each wrap).
    pub loop_iteration: u32,
}

/// Transport controls: playback position, tempo, time signature, loop region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transport {
    pub state: TransportState,
    /// Current playback position in frames (sample-accurate).
    pub position: FramePos,
    /// Tempo in BPM.
    pub bpm: f64,
    /// Time signature numerator (e.g. 4 in 4/4).
    pub time_sig_num: u8,
    /// Time signature denominator (e.g. 4 in 4/4).
    pub time_sig_den: u8,
    /// Sample rate of the session.
    pub sample_rate: u32,
    /// Loop enabled.
    pub loop_enabled: bool,
    /// Loop start position in frames.
    pub loop_start: FramePos,
    /// Loop end position in frames.
    pub loop_end: FramePos,
    /// Current loop iteration (0-indexed, increments on each wrap).
    #[serde(skip)]
    pub loop_iteration: u32,
}

impl Transport {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            state: TransportState::Stopped,
            position: FramePos::ZERO,
            bpm: 120.0,
            time_sig_num: 4,
            time_sig_den: 4,
            sample_rate,
            loop_enabled: false,
            loop_start: FramePos::ZERO,
            loop_end: FramePos::ZERO,
            loop_iteration: 0,
        }
    }

    pub fn play(&mut self) {
        self.state = TransportState::Playing;
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.position = FramePos::ZERO;
        self.loop_iteration = 0;
    }

    pub fn pause(&mut self) {
        self.state = TransportState::Stopped;
    }

    pub fn seek(&mut self, position: FramePos) {
        self.position = position;
    }

    /// Advance the transport by `frames` and return the actual range processed,
    /// handling loop boundaries.
    pub fn advance(&mut self, frames: u32) -> AdvanceResult {
        let start = self.position;
        let mut loop_wrapped = false;

        if self.loop_enabled && self.loop_end > self.loop_start {
            let end = start + FramePos::from(frames);
            if end >= self.loop_end {
                let loop_length = self.loop_end - self.loop_start;
                if loop_length == FramePos::ZERO {
                    self.position = self.loop_start;
                    return AdvanceResult {
                        start,
                        end: self.position,
                        loop_wrapped: false,
                        loop_iteration: self.loop_iteration,
                    };
                }
                let overshoot = end - self.loop_end;
                self.position = self.loop_start + (overshoot % loop_length);
                self.loop_iteration += 1;
                loop_wrapped = true;
            } else {
                self.position = end;
            }
        } else {
            self.position += FramePos::from(frames);
        }

        AdvanceResult {
            start,
            end: self.position,
            loop_wrapped,
            loop_iteration: self.loop_iteration,
        }
    }

    /// Convert a frame position to seconds.
    pub fn frames_to_secs(&self, frames: FramePos) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        frames.as_f64() / self.sample_rate as f64
    }

    /// Convert seconds to frame position.
    pub fn secs_to_frames(&self, secs: f64) -> FramePos {
        if secs <= 0.0 {
            return FramePos::ZERO;
        }
        let frames = secs * self.sample_rate as f64;
        if frames > u64::MAX as f64 {
            return FramePos(u64::MAX);
        }
        FramePos(frames as u64)
    }

    /// Convert a frame position to beats.
    pub fn frames_to_beats(&self, frames: FramePos) -> f64 {
        let secs = self.frames_to_secs(frames);
        secs * self.bpm / 60.0
    }

    /// Convert beats to frame position.
    pub fn beats_to_frames(&self, beats: f64) -> FramePos {
        if self.bpm <= 0.0 {
            return FramePos::ZERO;
        }
        let secs = beats * 60.0 / self.bpm;
        self.secs_to_frames(secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_advance() {
        let mut t = Transport::new(48000);
        t.play();
        let result = t.advance(256);
        assert_eq!(result.start, FramePos::ZERO);
        assert_eq!(result.end, FramePos(256));
        assert_eq!(t.position, FramePos(256));
        assert!(!result.loop_wrapped);
    }

    #[test]
    fn test_transport_loop() {
        let mut t = Transport::new(48000);
        t.loop_enabled = true;
        t.loop_start = FramePos::ZERO;
        t.loop_end = FramePos(1000);
        t.position = FramePos(900);
        t.play();

        let result = t.advance(256);
        assert_eq!(result.start, FramePos(900));
        // 900 + 256 = 1156, wraps to 1156 - 1000 = 156
        assert_eq!(t.position, FramePos(156));
        assert!(result.loop_wrapped);
        assert_eq!(result.loop_iteration, 1);
    }

    #[test]
    fn test_time_conversions() {
        let t = Transport::new(48000);
        assert_eq!(t.frames_to_secs(FramePos(48000)), 1.0);
        assert_eq!(t.secs_to_frames(1.0), FramePos(48000));

        // At 120 BPM: 1 beat = 0.5 seconds = 24000 frames
        assert!((t.frames_to_beats(FramePos(24000)) - 1.0).abs() < 1e-10);
        assert_eq!(t.beats_to_frames(1.0), FramePos(24000));
    }

    #[test]
    fn test_transport_loop_multi_overshoot() {
        // Advance far past loop_end (multiple loop lengths)
        let mut t = Transport::new(48000);
        t.loop_enabled = true;
        t.loop_start = FramePos(100);
        t.loop_end = FramePos(200); // loop_length = 100
        t.position = FramePos(190);
        t.play();

        // 190 + 256 = 446, overshoot = 446 - 200 = 246, 246 % 100 = 46
        // position = 100 + 46 = 146
        let result = t.advance(256);
        assert_eq!(t.position, FramePos(146));
        assert!(result.loop_wrapped);
    }

    #[test]
    fn test_transport_loop_exact_boundary() {
        let mut t = Transport::new(48000);
        t.loop_enabled = true;
        t.loop_start = FramePos::ZERO;
        t.loop_end = FramePos(256);
        t.position = FramePos::ZERO;
        t.play();

        // Advance exactly to loop_end
        let result = t.advance(256);
        assert_eq!(t.position, FramePos::ZERO);
        assert!(result.loop_wrapped);
    }

    #[test]
    fn test_loop_iteration_increments() {
        let mut t = Transport::new(48000);
        t.loop_enabled = true;
        t.loop_start = FramePos::ZERO;
        t.loop_end = FramePos(100);
        t.play();

        assert_eq!(t.loop_iteration, 0);

        // First loop wrap
        let r1 = t.advance(100);
        assert!(r1.loop_wrapped);
        assert_eq!(r1.loop_iteration, 1);
        assert_eq!(t.loop_iteration, 1);

        // Advance within loop (no wrap)
        let r2 = t.advance(50);
        assert!(!r2.loop_wrapped);
        assert_eq!(r2.loop_iteration, 1);

        // Second loop wrap
        let r3 = t.advance(60); // 50+60=110, wraps
        assert!(r3.loop_wrapped);
        assert_eq!(r3.loop_iteration, 2);
    }

    #[test]
    fn test_stop_resets_loop_iteration() {
        let mut t = Transport::new(48000);
        t.loop_enabled = true;
        t.loop_start = FramePos::ZERO;
        t.loop_end = FramePos(100);
        t.play();
        t.advance(100); // wrap once
        assert_eq!(t.loop_iteration, 1);

        t.stop();
        assert_eq!(t.loop_iteration, 0);
    }
}
