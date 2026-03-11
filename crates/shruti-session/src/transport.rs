use serde::{Deserialize, Serialize};

/// Transport playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
    Recording,
}

/// Transport controls: playback position, tempo, time signature, loop region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transport {
    pub state: TransportState,
    /// Current playback position in frames (sample-accurate).
    pub position: u64,
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
    pub loop_start: u64,
    /// Loop end position in frames.
    pub loop_end: u64,
}

impl Transport {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            state: TransportState::Stopped,
            position: 0,
            bpm: 120.0,
            time_sig_num: 4,
            time_sig_den: 4,
            sample_rate,
            loop_enabled: false,
            loop_start: 0,
            loop_end: 0,
        }
    }

    pub fn play(&mut self) {
        self.state = TransportState::Playing;
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.position = 0;
    }

    pub fn pause(&mut self) {
        self.state = TransportState::Stopped;
    }

    pub fn seek(&mut self, position: u64) {
        self.position = position;
    }

    /// Advance the transport by `frames` and return the actual range processed,
    /// handling loop boundaries.
    pub fn advance(&mut self, frames: u32) -> (u64, u64) {
        let start = self.position;

        if self.loop_enabled && self.loop_end > self.loop_start {
            let end = start + frames as u64;
            if end >= self.loop_end {
                let loop_length = self.loop_end - self.loop_start;
                let overshoot = end - self.loop_end;
                self.position = self.loop_start + (overshoot % loop_length);
            } else {
                self.position = end;
            }
        } else {
            self.position += frames as u64;
        }

        (start, self.position)
    }

    /// Convert a frame position to seconds.
    pub fn frames_to_secs(&self, frames: u64) -> f64 {
        frames as f64 / self.sample_rate as f64
    }

    /// Convert seconds to frame position.
    pub fn secs_to_frames(&self, secs: f64) -> u64 {
        (secs * self.sample_rate as f64) as u64
    }

    /// Convert a frame position to beats.
    pub fn frames_to_beats(&self, frames: u64) -> f64 {
        let secs = self.frames_to_secs(frames);
        secs * self.bpm / 60.0
    }

    /// Convert beats to frame position.
    pub fn beats_to_frames(&self, beats: f64) -> u64 {
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
        let (start, end) = t.advance(256);
        assert_eq!(start, 0);
        assert_eq!(end, 256);
        assert_eq!(t.position, 256);
    }

    #[test]
    fn test_transport_loop() {
        let mut t = Transport::new(48000);
        t.loop_enabled = true;
        t.loop_start = 0;
        t.loop_end = 1000;
        t.position = 900;
        t.play();

        let (start, _end) = t.advance(256);
        assert_eq!(start, 900);
        // 900 + 256 = 1156, wraps to 1156 - 1000 = 156
        assert_eq!(t.position, 156);
    }

    #[test]
    fn test_time_conversions() {
        let t = Transport::new(48000);
        assert_eq!(t.frames_to_secs(48000), 1.0);
        assert_eq!(t.secs_to_frames(1.0), 48000);

        // At 120 BPM: 1 beat = 0.5 seconds = 24000 frames
        assert!((t.frames_to_beats(24000) - 1.0).abs() < 1e-10);
        assert_eq!(t.beats_to_frames(1.0), 24000);
    }

    #[test]
    fn test_transport_loop_multi_overshoot() {
        // Advance far past loop_end (multiple loop lengths)
        let mut t = Transport::new(48000);
        t.loop_enabled = true;
        t.loop_start = 100;
        t.loop_end = 200; // loop_length = 100
        t.position = 190;
        t.play();

        // 190 + 256 = 446, overshoot = 446 - 200 = 246, 246 % 100 = 46
        // position = 100 + 46 = 146
        t.advance(256);
        assert_eq!(t.position, 146);
    }

    #[test]
    fn test_transport_loop_exact_boundary() {
        let mut t = Transport::new(48000);
        t.loop_enabled = true;
        t.loop_start = 0;
        t.loop_end = 256;
        t.position = 0;
        t.play();

        // Advance exactly to loop_end
        t.advance(256);
        assert_eq!(t.position, 0);
    }
}
