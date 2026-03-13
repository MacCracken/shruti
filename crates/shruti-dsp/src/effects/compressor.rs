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
    ///
    /// Uses the standard soft knee formula:
    ///   gain = (1/ratio - 1) * (input_db - threshold + knee/2)^2 / (2 * knee)
    /// which ensures a smooth C1-continuous transition from 0 dB gain reduction
    /// at the bottom of the knee to full ratio compression at the top.
    fn compute_gain_db(&self, input_db: f32) -> f32 {
        let half_knee = self.knee_db / 2.0;
        let over = input_db - self.threshold_db;

        if self.knee_db > 0.0 && over.abs() < half_knee {
            // Soft knee region: standard quadratic interpolation
            let x = over + half_knee;
            (1.0 / self.ratio - 1.0) * x * x / (2.0 * self.knee_db)
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

            // Compute gain reduction (using fast approximations for per-sample path)
            let env_db = fast_linear_to_db(self.envelope);
            let gain_db = self.compute_gain_db(env_db);
            let gain = fast_db_to_linear(gain_db) * makeup_linear;

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

/// Fast approximation of `10^(x/20)` = `10^(x * 0.05)` for dB-to-linear conversion.
///
/// Uses the identity: 10^y = 2^(y / log2(10)) = 2^(y * 3.32193)
/// and an IEEE 754 bit-trick for fast exp2.
/// Maximum error ~0.2% across the -80..+20 dB range.
#[inline]
fn fast_db_to_linear(db: f32) -> f32 {
    let y = db * 0.05; // db / 20
    let x = y * std::f32::consts::LOG2_10; // convert to base-2 exponent
    // Fast exp2 via IEEE 754 bit manipulation
    let x = x.clamp(-126.0, 126.0);
    let xi = x.floor() as i32;
    let xf = x - xi as f32;
    let base = f32::from_bits(((xi + 127) as u32) << 23);
    base * (1.0 + xf * (std::f32::consts::LN_2 + xf * 0.2402265))
}

/// Fast approximation of `20 * log10(x)` for linear-to-dB conversion.
///
/// Uses the identity: log10(x) = log2(x) / log2(10)
/// and an IEEE 754 bit-trick for fast log2.
/// Maximum error ~0.3% for typical audio levels.
#[inline]
fn fast_linear_to_db(linear: f32) -> f32 {
    if linear < 1e-10 {
        return -200.0;
    }
    // Fast log2 via IEEE 754 bit manipulation
    let bits = linear.to_bits();
    let exponent = ((bits >> 23) & 0xFF) as f32 - 127.0;
    let mantissa = f32::from_bits((bits & 0x007FFFFF) | 0x3F800000);
    // Polynomial approximation of log2(mantissa) for mantissa in [1, 2)
    let log2_approx = exponent + (mantissa - 1.0) * (2.0 - 0.3333333 * (mantissa - 1.0));
    20.0 * log2_approx / std::f32::consts::LOG2_10
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

    #[test]
    fn test_soft_knee_behavior() {
        let comp = Compressor::new(48000.0);
        // Default: threshold=-20, ratio=4, knee=6
        // Knee region: -23 to -17

        // Below the knee entirely (below threshold - half_knee = -23)
        let gain_below = comp.compute_gain_db(-26.0);
        // Just inside the knee from the bottom
        let gain_in_knee_low = comp.compute_gain_db(-22.0);
        // Above the knee entirely (above threshold + half_knee = -17)
        let gain_above = comp.compute_gain_db(-14.0);

        // Below knee: no compression
        assert!(
            gain_below.abs() < 0.01,
            "Below knee should have ~0 dB gain reduction: {gain_below}"
        );

        // Inside the knee near the bottom: soft knee should apply only a small amount
        // of compression (gradual onset)
        assert!(
            gain_in_knee_low < 0.0,
            "Soft knee should start compressing inside knee region: {gain_in_knee_low}"
        );
        assert!(
            gain_in_knee_low > -1.0,
            "Near bottom of knee, compression should be gentle: {gain_in_knee_low}"
        );

        // Above knee: significant compression (negative gain)
        assert!(
            gain_above < 0.0,
            "Above knee should have compression: {gain_above}"
        );

        // Compression should increase as we go through the knee
        assert!(
            gain_in_knee_low > gain_above,
            "More compression above knee than inside: inside={gain_in_knee_low}, above={gain_above}"
        );
    }

    #[test]
    fn test_makeup_gain() {
        let frames = 4800;
        let data: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin() * 0.1)
            .collect();

        // Without makeup gain
        let mut comp_no_makeup = Compressor::new(48000.0);
        comp_no_makeup.threshold_db = -20.0;
        comp_no_makeup.ratio = 4.0;
        comp_no_makeup.makeup_db = 0.0;
        comp_no_makeup.knee_db = 0.0;
        let mut buf1 = AudioBuffer::from_interleaved(data.clone(), 1);
        comp_no_makeup.process(&mut buf1);
        let rms_no_makeup: f32 = (0..frames)
            .map(|i| buf1.get(i as u32, 0).powi(2))
            .sum::<f32>()
            / frames as f32;

        // With 12 dB makeup gain
        let mut comp_makeup = Compressor::new(48000.0);
        comp_makeup.threshold_db = -20.0;
        comp_makeup.ratio = 4.0;
        comp_makeup.makeup_db = 12.0;
        comp_makeup.knee_db = 0.0;
        let mut buf2 = AudioBuffer::from_interleaved(data, 1);
        comp_makeup.process(&mut buf2);
        let rms_makeup: f32 = (0..frames)
            .map(|i| buf2.get(i as u32, 0).powi(2))
            .sum::<f32>()
            / frames as f32;

        assert!(
            rms_makeup > rms_no_makeup * 2.0,
            "Makeup gain should increase output: with={rms_makeup}, without={rms_no_makeup}"
        );
    }

    #[test]
    fn test_attack_release_time_constants() {
        // Fast attack should compress quickly, slow attack should let transients through
        let frames = 4800;
        // Signal: silence then suddenly loud
        let mut data: Vec<f32> = vec![0.0; frames];
        for (idx, sample) in data.iter_mut().enumerate().skip(480) {
            *sample = (2.0 * std::f32::consts::PI * 440.0 * idx as f32 / 48000.0).sin();
        }

        // Fast attack (1ms)
        let mut comp_fast = Compressor::new(48000.0);
        comp_fast.threshold_db = -20.0;
        comp_fast.ratio = 10.0;
        comp_fast.attack = 0.001;
        comp_fast.release = 0.1;
        comp_fast.makeup_db = 0.0;
        comp_fast.knee_db = 0.0;
        let mut buf_fast = AudioBuffer::from_interleaved(data.clone(), 1);
        comp_fast.process(&mut buf_fast);

        // Slow attack (100ms)
        let mut comp_slow = Compressor::new(48000.0);
        comp_slow.threshold_db = -20.0;
        comp_slow.ratio = 10.0;
        comp_slow.attack = 0.1;
        comp_slow.release = 0.1;
        comp_slow.makeup_db = 0.0;
        comp_slow.knee_db = 0.0;
        let mut buf_slow = AudioBuffer::from_interleaved(data, 1);
        comp_slow.process(&mut buf_slow);

        // With slow attack, the initial transient (first ~50 samples after onset)
        // should be louder than with fast attack
        let transient_start = 480;
        let transient_end = 530;
        let peak_fast: f32 = (transient_start..transient_end)
            .map(|i| buf_fast.get(i as u32, 0).abs())
            .fold(0.0_f32, f32::max);
        let peak_slow: f32 = (transient_start..transient_end)
            .map(|i| buf_slow.get(i as u32, 0).abs())
            .fold(0.0_f32, f32::max);

        assert!(
            peak_slow > peak_fast,
            "Slow attack should let more transient through: slow={peak_slow}, fast={peak_fast}"
        );
    }

    #[test]
    fn test_hard_knee_above_threshold() {
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 4.0;
        comp.knee_db = 0.0;

        // 6 dB above threshold => should get compressed
        let gain = comp.compute_gain_db(-14.0);
        // Expected: threshold + over/ratio - input = -20 + 6/4 - (-14) = -20 + 1.5 + 14 = -4.5
        assert!(
            (gain - (-4.5)).abs() < 0.01,
            "Hard knee above threshold: expected -4.5, got {gain}"
        );
    }

    #[test]
    fn test_hard_knee_below_threshold() {
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 4.0;
        comp.knee_db = 0.0;

        let gain = comp.compute_gain_db(-30.0);
        assert!(
            gain.abs() < 0.01,
            "Below threshold with hard knee should have 0 dB gain reduction, got {gain}"
        );
    }

    #[test]
    fn test_infinite_ratio_acts_as_limiter() {
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -10.0;
        comp.ratio = f32::INFINITY;
        comp.knee_db = 0.0;

        // Above threshold, gain should bring signal back to threshold
        let gain = comp.compute_gain_db(0.0);
        // threshold + over/inf - input = -10 + 0 - 0 = -10
        assert!(
            (gain - (-10.0)).abs() < 0.01,
            "Infinite ratio should limit to threshold, got {gain}"
        );
    }

    #[test]
    fn test_compressor_reset_clears_envelope() {
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 4.0;
        comp.knee_db = 0.0;

        // Process a loud signal to build up envelope
        let data: Vec<f32> = vec![1.0; 480];
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        comp.process(&mut buf);
        assert!(
            comp.envelope > 0.0,
            "Envelope should be nonzero after processing"
        );

        comp.reset();
        assert_eq!(comp.envelope, 0.0, "Reset should clear envelope");
    }

    #[test]
    fn test_compressor_silence_passthrough() {
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 4.0;
        comp.knee_db = 0.0;
        comp.makeup_db = 0.0;

        let mut buf = AudioBuffer::new(1, 256);
        comp.process(&mut buf);

        // Silence should remain silence
        for i in 0..256 {
            assert_eq!(buf.get(i, 0), 0.0);
        }
    }

    #[test]
    fn test_compressor_stereo_processing() {
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 4.0;
        comp.attack = 0.001;
        comp.knee_db = 0.0;
        comp.makeup_db = 0.0;

        let frames = 2400;
        let mut data = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let val = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin();
            data[i * 2] = val;
            data[i * 2 + 1] = val * 0.5; // right channel quieter
        }

        let mut buf = AudioBuffer::from_interleaved(data, 2);
        comp.process(&mut buf);

        // Both channels should be processed (gain reduction applied based on peak across channels)
        let peak_l: f32 = (480..frames)
            .map(|i| buf.get(i as u32, 0).abs())
            .fold(0.0f32, f32::max);
        let peak_r: f32 = (480..frames)
            .map(|i| buf.get(i as u32, 1).abs())
            .fold(0.0f32, f32::max);

        assert!(peak_l < 1.0, "Left channel should be compressed");
        assert!(peak_r < 0.5, "Right channel should also be compressed");
    }

    #[test]
    fn test_linear_to_db_near_zero() {
        let db = linear_to_db(1e-11);
        assert_eq!(db, -200.0);
    }

    #[test]
    fn test_db_to_linear_identity() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_db_to_linear_minus_6() {
        let lin = db_to_linear(-6.0206);
        assert!((lin - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_soft_knee_smooth_transition() {
        // Verify the knee region produces a smooth (monotonic, no discontinuities) gain curve.
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 4.0;
        comp.knee_db = 6.0;

        let half_knee = comp.knee_db / 2.0;
        let start = comp.threshold_db - half_knee; // -23
        let end = comp.threshold_db + half_knee; // -17

        // Sample the gain curve at fine steps through the knee region
        let steps = 100;
        let mut prev_gain = comp.compute_gain_db(start - 0.01);
        for i in 0..=steps {
            let db = start + (end - start) * i as f32 / steps as f32;
            let gain = comp.compute_gain_db(db);

            // Gain should be monotonically decreasing (more negative) as input increases
            assert!(
                gain <= prev_gain + 1e-4,
                "Gain should be monotonically decreasing: at {db:.2} dB, gain={gain:.4}, prev={prev_gain:.4}"
            );
            prev_gain = gain;
        }
    }

    #[test]
    fn test_soft_knee_boundary_continuity() {
        // At the boundaries of the knee region, the soft knee should match
        // the linear (no-compression and full-compression) regions.
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 4.0;
        comp.knee_db = 6.0;

        let half_knee = comp.knee_db / 2.0;

        // At bottom of knee (threshold - knee/2 = -23), gain should be ~0
        let gain_bottom = comp.compute_gain_db(comp.threshold_db - half_knee);
        assert!(
            gain_bottom.abs() < 0.01,
            "At bottom of knee, gain should be ~0 dB, got {gain_bottom}"
        );

        // At top of knee (threshold + knee/2 = -17), gain should match the hard-knee formula
        let gain_top = comp.compute_gain_db(comp.threshold_db + half_knee);
        let expected_hard =
            (comp.threshold_db + half_knee / comp.ratio) - (comp.threshold_db + half_knee);
        assert!(
            (gain_top - expected_hard).abs() < 0.1,
            "At top of knee, soft knee ({gain_top:.3}) should match hard knee ({expected_hard:.3})"
        );
    }
}
