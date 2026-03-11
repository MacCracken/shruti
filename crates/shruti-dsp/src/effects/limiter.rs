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
}
