use crate::buffer::AudioBuffer;

/// Brickwall limiter — prevents signal from exceeding the ceiling.
///
/// Uses a fast-attack, slow-release envelope to smoothly limit peaks.
#[derive(Debug, Clone)]
pub struct Limiter {
    /// Ceiling in dB (maximum output level).
    pub ceiling_db: f32,
    /// Release time in seconds.
    pub release: f32,
    sample_rate: f32,
    envelope: f32,
}

impl Limiter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            ceiling_db: -0.3,
            release: 0.1,
            sample_rate,
            envelope: 1.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Process an audio buffer in place.
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        let channels = buffer.channels() as usize;
        let frames = buffer.frames();
        let ceiling = 10.0_f32.powf(self.ceiling_db / 20.0);
        let release_coeff = (-1.0 / (self.release * self.sample_rate)).exp();

        for frame in 0..frames {
            // Detect peak across channels
            let mut peak: f32 = 0.0;
            for ch in 0..channels {
                peak = peak.max(buffer.get(frame, ch as u16).abs());
            }

            // Compute required gain reduction
            let target = if peak > ceiling { ceiling / peak } else { 1.0 };

            // Envelope: instant attack, smooth release
            if target < self.envelope {
                self.envelope = target;
            } else {
                self.envelope = release_coeff * self.envelope + (1.0 - release_coeff) * target;
            }

            // Apply gain
            for ch in 0..channels {
                let sample = buffer.get(frame, ch as u16) * self.envelope;
                buffer.set(frame, ch as u16, sample);
            }
        }
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.envelope = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_caps_output() {
        let mut limiter = Limiter::new(48000.0);
        limiter.ceiling_db = -6.0; // ~0.5 linear
        limiter.release = 0.001;

        let ceiling_linear = 10.0_f32.powf(-6.0 / 20.0);

        // Loud signal
        let frames = 2400;
        let data: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();

        let mut buf = AudioBuffer::from_interleaved(data, 1);
        limiter.process(&mut buf);

        // After settling, peaks should not exceed ceiling
        for i in 480..frames {
            let sample = buf.get(i as u32, 0).abs();
            assert!(
                sample <= ceiling_linear + 0.05,
                "frame {i}: {sample} exceeds ceiling {ceiling_linear}"
            );
        }
    }

    #[test]
    fn test_limiter_quiet_passthrough() {
        let mut limiter = Limiter::new(48000.0);
        limiter.ceiling_db = 0.0;

        // Signal well below ceiling
        let data = vec![0.1_f32; 256];
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        limiter.process(&mut buf);

        // Should pass through with minimal change (envelope settling)
        for i in 10..256 {
            assert!((buf.get(i, 0) - 0.1).abs() < 0.05);
        }
    }

    #[test]
    fn test_limiter_silence_passthrough() {
        let mut limiter = Limiter::new(48000.0);
        limiter.ceiling_db = 0.0;
        let mut buf = AudioBuffer::new(2, 256);
        limiter.process(&mut buf);
        for i in 0..256 {
            assert_eq!(buf.get(i, 0), 0.0);
            assert_eq!(buf.get(i, 1), 0.0);
        }
    }

    #[test]
    fn test_limiter_various_ceilings() {
        for &ceiling_db in &[-12.0, -6.0, -3.0, -1.0, 0.0] {
            let mut limiter = Limiter::new(48000.0);
            limiter.ceiling_db = ceiling_db;
            limiter.release = 0.001;

            let ceiling_linear = 10.0_f32.powf(ceiling_db / 20.0);
            let frames = 2400;
            let data: Vec<f32> = (0..frames)
                .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
                .collect();
            let mut buf = AudioBuffer::from_interleaved(data, 1);
            limiter.process(&mut buf);

            for i in 480..frames {
                let sample = buf.get(i as u32, 0).abs();
                assert!(
                    sample <= ceiling_linear + 0.05,
                    "ceiling_db={ceiling_db}: frame {i}: {sample} exceeds ceiling {ceiling_linear}"
                );
            }
        }
    }

    #[test]
    fn test_limiter_stereo_limiting() {
        let mut limiter = Limiter::new(48000.0);
        limiter.ceiling_db = -6.0;
        limiter.release = 0.001;
        let ceiling_linear = 10.0_f32.powf(-6.0 / 20.0);

        let frames = 2400;
        let mut data = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let val = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin();
            data[i * 2] = val;
            data[i * 2 + 1] = val * 0.8;
        }
        let mut buf = AudioBuffer::from_interleaved(data, 2);
        limiter.process(&mut buf);

        for i in 480..frames {
            let l = buf.get(i as u32, 0).abs();
            let r = buf.get(i as u32, 1).abs();
            assert!(
                l <= ceiling_linear + 0.05,
                "L frame {i}: {l} exceeds ceiling"
            );
            assert!(
                r <= ceiling_linear + 0.05,
                "R frame {i}: {r} exceeds ceiling"
            );
        }
    }

    #[test]
    fn test_limiter_reset() {
        let mut limiter = Limiter::new(48000.0);
        limiter.ceiling_db = -6.0;

        // Process loud signal
        let data: Vec<f32> = vec![1.0; 480];
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        limiter.process(&mut buf);
        assert!(limiter.envelope < 1.0, "Envelope should be reduced");

        limiter.reset();
        assert_eq!(
            limiter.envelope, 1.0,
            "Reset should restore envelope to 1.0"
        );
    }

    #[test]
    fn test_limiter_below_ceiling_minimal_change() {
        let mut limiter = Limiter::new(48000.0);
        limiter.ceiling_db = 0.0; // ceiling at 1.0 linear

        // Signal at 0.3 -- well below ceiling
        let frames = 480;
        let data: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin() * 0.3)
            .collect();
        let original = data.clone();
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        limiter.process(&mut buf);

        // Output should be very close to input
        for (i, orig) in original.iter().enumerate().take(frames).skip(10) {
            let diff = (buf.get(i as u32, 0) - orig).abs();
            assert!(
                diff < 0.05,
                "Below ceiling, frame {i} should be ~unchanged, diff={diff}"
            );
        }
    }
}
