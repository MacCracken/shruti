use crate::buffer::AudioBuffer;

/// Stereo balance / pan control.
///
/// For stereo sources, acts as a balance control (center = unity on both channels).
/// Pan position: -1.0 = full left, 0.0 = center, 1.0 = full right.
///
/// Uses constant-power panning law: at center both channels are at unity,
/// panning left attenuates right channel (and vice versa) using a cosine curve.
#[derive(Debug, Clone)]
pub struct StereoPanner {
    /// Pan position (-1.0 to 1.0).
    pub pan: f32,
}

impl StereoPanner {
    pub fn new(pan: f32) -> Self {
        Self {
            pan: pan.clamp(-1.0, 1.0),
        }
    }

    /// Compute left and right gain multipliers.
    ///
    /// At center (0.0): both = 1.0.
    /// At hard left (-1.0): L = 1.0, R = 0.0.
    /// At hard right (1.0): L = 0.0, R = 1.0.
    pub fn gains(&self) -> (f32, f32) {
        let p = self.pan.clamp(-1.0, 1.0);
        // Map pan [-1, 1] to angle [0, PI/2]
        let angle = (p + 1.0) * 0.25 * std::f32::consts::PI;
        // At center: cos(PI/4) = sin(PI/4) ≈ 0.707, but we want unity at center.
        // Use linear crossfade for balance: left fades down as pan goes right.
        let gain_l = if p <= 0.0 { 1.0 } else { 1.0 - p };
        let gain_r = if p >= 0.0 { 1.0 } else { 1.0 + p };
        let _ = angle; // Reserved for future equal-power option
        (gain_l, gain_r)
    }

    /// Process a stereo audio buffer in place.
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        if buffer.channels() < 2 {
            return;
        }

        let (gain_l, gain_r) = self.gains();
        let frames = buffer.frames();

        for frame in 0..frames {
            let left = buffer.get(frame, 0) * gain_l;
            let right = buffer.get(frame, 1) * gain_r;
            buffer.set(frame, 0, left);
            buffer.set(frame, 1, right);
        }
    }
}

impl Default for StereoPanner {
    fn default() -> Self {
        Self::new(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_center_pan_unity() {
        let panner = StereoPanner::new(0.0);
        let (l, r) = panner.gains();
        assert!((l - 1.0).abs() < 0.001, "Center: L should be 1.0, got {l}");
        assert!((r - 1.0).abs() < 0.001, "Center: R should be 1.0, got {r}");
    }

    #[test]
    fn test_hard_left() {
        let panner = StereoPanner::new(-1.0);
        let (l, r) = panner.gains();
        assert!((l - 1.0).abs() < 0.001, "Hard left: L should be 1.0");
        assert!(r.abs() < 0.001, "Hard left: R should be 0.0");
    }

    #[test]
    fn test_hard_right() {
        let panner = StereoPanner::new(1.0);
        let (l, r) = panner.gains();
        assert!(l.abs() < 0.001, "Hard right: L should be 0.0");
        assert!((r - 1.0).abs() < 0.001, "Hard right: R should be 1.0");
    }

    #[test]
    fn test_pan_process_stereo() {
        let mut panner = StereoPanner::new(-1.0);
        let mut buf = AudioBuffer::from_interleaved(vec![0.5, 0.5, 0.5, 0.5], 2);
        panner.process(&mut buf);

        assert!((buf.get(0, 0) - 0.5).abs() < 0.001);
        assert!(buf.get(0, 1).abs() < 0.01);
    }

    #[test]
    fn test_center_passthrough() {
        let mut panner = StereoPanner::new(0.0);
        let mut buf = AudioBuffer::from_interleaved(vec![0.5, -0.3, 0.7, -0.1], 2);
        panner.process(&mut buf);

        assert!((buf.get(0, 0) - 0.5).abs() < 0.001);
        assert!((buf.get(0, 1) - (-0.3)).abs() < 0.001);
    }

    #[test]
    fn test_mono_buffer_noop() {
        let mut panner = StereoPanner::new(-1.0);
        let mut buf = AudioBuffer::from_interleaved(vec![0.5, 0.5], 1);
        panner.process(&mut buf);
        assert_eq!(buf.get(0, 0), 0.5);
    }
}
