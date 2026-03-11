use crate::buffer::AudioBuffer;
use crate::format::Sample;

/// Dynamic range compressor with adjustable threshold, ratio, attack, and release.
#[derive(Debug, Clone)]
pub struct Compressor {
    /// Threshold in dB (signals above this are compressed).
    pub threshold_db: f32,
    /// Compression ratio (e.g., 4.0 means 4:1).
    pub ratio: f32,
    /// Attack time in seconds.
    pub attack: f32,
    /// Release time in seconds.
    pub release: f32,
    /// Makeup gain in dB.
    pub makeup_db: f32,
    /// Knee width in dB (0 = hard knee).
    pub knee_db: f32,
    sample_rate: f32,
    /// Envelope follower state (linear).
    envelope: f32,
}

impl Compressor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            threshold_db: -20.0,
            ratio: 4.0,
            attack: 0.01,
            release: 0.1,
            makeup_db: 0.0,
            knee_db: 6.0,
            sample_rate,
            envelope: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Compute gain reduction in dB for a given input level in dB.
    fn compute_gain_db(&self, input_db: f32) -> f32 {
        let half_knee = self.knee_db / 2.0;
        let over = input_db - self.threshold_db;

        if self.knee_db > 0.0 && over.abs() < half_knee {
            // Soft knee region
            let x = over + half_knee;
            let compressed = self.threshold_db + x - x * x / (2.0 * self.knee_db)
                + (x * x / (2.0 * self.knee_db)) / self.ratio;
            compressed - input_db
        } else if over <= -half_knee {
            // Below threshold
            0.0
        } else {
            // Above threshold
            (self.threshold_db + over / self.ratio) - input_db
        }
    }

    /// Process an audio buffer in place.
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        let channels = buffer.channels() as usize;
        let frames = buffer.frames();
        let attack_coeff = (-1.0 / (self.attack * self.sample_rate)).exp();
        let release_coeff = (-1.0 / (self.release * self.sample_rate)).exp();
        let makeup_linear = db_to_linear(self.makeup_db);

        for frame in 0..frames {
            // Detect peak across channels
            let mut peak: f32 = 0.0;
            for ch in 0..channels {
                peak = peak.max(buffer.get(frame, ch as u16).abs());
            }

            // Envelope follower
            let coeff = if peak > self.envelope {
                attack_coeff
            } else {
                release_coeff
            };
            self.envelope = coeff * self.envelope + (1.0 - coeff) * peak;

            // Compute gain reduction
            let env_db = linear_to_db(self.envelope);
            let gain_db = self.compute_gain_db(env_db);
            let gain = db_to_linear(gain_db) * makeup_linear;

            // Apply gain
            for ch in 0..channels {
                let sample = buffer.get(frame, ch as u16) * gain;
                buffer.set(frame, ch as u16, sample);
            }
        }
    }

    /// Reset envelope state.
    pub fn reset(&mut self) {
        self.envelope = 0.0;
    }
}

fn linear_to_db(linear: f32) -> f32 {
    if linear < 1e-10 {
        -200.0
    } else {
        20.0 * linear.log10()
    }
}

fn db_to_linear(db: f32) -> Sample {
    10.0_f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_below_threshold() {
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = 0.0;
        comp.ratio = 4.0;
        comp.makeup_db = 0.0;
        comp.knee_db = 0.0;

        // Quiet signal should pass through mostly unchanged
        let data: Vec<f32> = (0..256).map(|_| 0.01).collect();
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        comp.process(&mut buf);

        for i in 10..256 {
            let sample = buf.get(i, 0);
            assert!((sample - 0.01).abs() < 0.005, "frame {i}: {sample}");
        }
    }

    #[test]
    fn test_compressor_reduces_loud_signal() {
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 10.0;
        comp.attack = 0.001;
        comp.release = 0.01;
        comp.makeup_db = 0.0;
        comp.knee_db = 0.0;

        // Loud signal (0 dBFS)
        let frames = 4800;
        let data: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();
        let peak_before = data.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);

        let mut buf = AudioBuffer::from_interleaved(data, 1);
        comp.process(&mut buf);

        // After compression, peak should be lower
        let peak_after = (0..frames)
            .map(|i| buf.get(i as u32, 0).abs())
            .fold(0.0_f32, f32::max);

        assert!(
            peak_after < peak_before,
            "Compressed peak {peak_after} should be less than original {peak_before}"
        );
    }

    #[test]
    fn test_db_conversion_roundtrip() {
        let db = -6.0_f32;
        let linear = db_to_linear(db);
        let back = linear_to_db(linear);
        assert!((back - db).abs() < 0.001);
    }
}
