use crate::buffer::AudioBuffer;
use crate::format::Sample;

/// Filter type for a single EQ band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    LowShelf,
    HighShelf,
    Peak,
    LowPass,
    HighPass,
}

/// A single EQ band with biquad filter coefficients.
#[derive(Debug, Clone)]
pub struct EqBand {
    pub filter_type: FilterType,
    pub frequency: f32,
    pub gain_db: f32,
    pub q: f32,
    pub enabled: bool,
    // Biquad coefficients
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    // Per-channel state (up to 2 channels)
    state: [[f64; 2]; 2],
}

impl EqBand {
    pub fn new(filter_type: FilterType, frequency: f32, gain_db: f32, q: f32) -> Self {
        let mut band = Self {
            filter_type,
            frequency,
            gain_db,
            q,
            enabled: true,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            state: [[0.0; 2]; 2],
        };
        band.compute_coefficients(48000.0);
        band
    }

    /// Recompute biquad coefficients for the given sample rate.
    pub fn compute_coefficients(&mut self, sample_rate: f32) {
        let w0 = 2.0 * std::f64::consts::PI * self.frequency as f64 / sample_rate as f64;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * self.q as f64);
        let a = 10.0_f64.powf(self.gain_db as f64 / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match self.filter_type {
            FilterType::Peak => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowPass => {
                let b0 = (1.0 - cos_w0) / 2.0;
                let b1 = 1.0 - cos_w0;
                let b2 = (1.0 - cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_w0) / 2.0;
                let b1 = -(1.0 + cos_w0);
                let b2 = (1.0 + cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// Process a single sample on the given channel. Direct Form II Transposed.
    fn process_sample(&mut self, input: f64, channel: usize) -> f64 {
        let output = self.b0 * input + self.state[channel][0];
        self.state[channel][0] = self.b1 * input - self.a1 * output + self.state[channel][1];
        self.state[channel][1] = self.b2 * input - self.a2 * output;
        output
    }

    /// Reset filter state.
    pub fn reset(&mut self) {
        self.state = [[0.0; 2]; 2];
    }
}

/// Multi-band parametric equalizer.
#[derive(Debug, Clone)]
pub struct ParametricEq {
    pub bands: Vec<EqBand>,
    sample_rate: f32,
}

impl ParametricEq {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            bands: Vec::new(),
            sample_rate,
        }
    }

    /// Add a band and recompute its coefficients.
    pub fn add_band(&mut self, mut band: EqBand) {
        band.compute_coefficients(self.sample_rate);
        self.bands.push(band);
    }

    /// Update sample rate and recompute all coefficients.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for band in &mut self.bands {
            band.compute_coefficients(sample_rate);
        }
    }

    /// Process an audio buffer in place.
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        let channels = buffer.channels() as usize;
        let frames = buffer.frames();

        for frame in 0..frames {
            for ch in 0..channels.min(2) {
                let mut sample = buffer.get(frame, ch as u16) as f64;
                for band in &mut self.bands {
                    if band.enabled {
                        sample = band.process_sample(sample, ch);
                    }
                }
                buffer.set(frame, ch as u16, sample as Sample);
            }
        }
    }

    /// Reset all band states.
    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_passthrough() {
        // A peak band with 0 dB gain should pass through unchanged
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::Peak, 1000.0, 0.0, 1.0));

        let mut buf = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3], 2);
        eq.process(&mut buf);

        // With 0 dB gain, output should be very close to input
        assert!((buf.get(0, 0) - 0.5).abs() < 0.01);
        assert!((buf.get(0, 1) + 0.5).abs() < 0.01);
    }

    #[test]
    fn test_eq_boost() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::Peak, 1000.0, 12.0, 1.0));

        // Generate a 1kHz sine at 48kHz, stereo
        let frames = 4800;
        let mut data = vec![0.0_f32; frames * 2];
        for i in 0..frames {
            let val = (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin() * 0.5;
            data[i * 2] = val;
            data[i * 2 + 1] = val;
        }
        let mut buf = AudioBuffer::from_interleaved(data, 2);

        // Measure RMS before
        let rms_before: f32 = (0..frames)
            .map(|i| buf.get(i as u32, 0).powi(2))
            .sum::<f32>()
            / frames as f32;

        eq.process(&mut buf);

        // Measure RMS after — should be louder with 12dB boost at 1kHz
        let rms_after: f32 = (0..frames)
            .map(|i| buf.get(i as u32, 0).powi(2))
            .sum::<f32>()
            / frames as f32;

        assert!(
            rms_after > rms_before * 2.0,
            "12dB boost should increase level"
        );
    }

    #[test]
    fn test_disabled_band_passthrough() {
        let mut eq = ParametricEq::new(48000.0);
        let mut band = EqBand::new(FilterType::Peak, 1000.0, 12.0, 1.0);
        band.enabled = false;
        eq.add_band(band);

        let mut buf = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3], 2);
        eq.process(&mut buf);

        assert_eq!(buf.get(0, 0), 0.5);
        assert_eq!(buf.get(0, 1), -0.5);
    }

    fn generate_sine(freq: f32, sample_rate: f32, frames: usize, amplitude: f32) -> Vec<f32> {
        (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin() * amplitude)
            .collect()
    }

    fn rms_of_buffer(buf: &AudioBuffer, channel: u16, frames: usize) -> f32 {
        let sum: f32 = (0..frames)
            .map(|i| buf.get(i as u32, channel).powi(2))
            .sum();
        (sum / frames as f32).sqrt()
    }

    #[test]
    fn test_low_shelf_boosts_low_frequencies() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::LowShelf, 500.0, 12.0, 0.707));

        let frames = 4800;
        // Low frequency tone (100 Hz) should be boosted
        let data = generate_sine(100.0, 48000.0, frames, 0.3);
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        let rms_before = rms_of_buffer(&buf, 0, frames);
        eq.process(&mut buf);
        let rms_after = rms_of_buffer(&buf, 0, frames);
        assert!(
            rms_after > rms_before * 1.5,
            "Low shelf should boost 100 Hz: before={rms_before}, after={rms_after}"
        );
    }

    #[test]
    fn test_high_shelf_boosts_high_frequencies() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::HighShelf, 2000.0, 12.0, 0.707));

        let frames = 4800;
        // High frequency tone (10 kHz) should be boosted
        let data = generate_sine(10000.0, 48000.0, frames, 0.3);
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        let rms_before = rms_of_buffer(&buf, 0, frames);
        eq.process(&mut buf);
        let rms_after = rms_of_buffer(&buf, 0, frames);
        assert!(
            rms_after > rms_before * 1.5,
            "High shelf should boost 10 kHz: before={rms_before}, after={rms_after}"
        );
    }

    #[test]
    fn test_lowpass_attenuates_high_frequencies() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::LowPass, 500.0, 0.0, 0.707));

        let frames = 4800;
        let data = generate_sine(5000.0, 48000.0, frames, 0.5);
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        let rms_before = rms_of_buffer(&buf, 0, frames);
        eq.process(&mut buf);
        let rms_after = rms_of_buffer(&buf, 0, frames);
        assert!(
            rms_after < rms_before * 0.3,
            "LowPass at 500Hz should strongly attenuate 5kHz: before={rms_before}, after={rms_after}"
        );
    }

    #[test]
    fn test_highpass_attenuates_low_frequencies() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::HighPass, 5000.0, 0.0, 0.707));

        let frames = 4800;
        let data = generate_sine(100.0, 48000.0, frames, 0.5);
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        let rms_before = rms_of_buffer(&buf, 0, frames);
        eq.process(&mut buf);
        let rms_after = rms_of_buffer(&buf, 0, frames);
        assert!(
            rms_after < rms_before * 0.3,
            "HighPass at 5kHz should strongly attenuate 100Hz: before={rms_before}, after={rms_after}"
        );
    }

    #[test]
    fn test_enabling_disabling_bands() {
        let mut eq = ParametricEq::new(48000.0);
        let mut band = EqBand::new(FilterType::Peak, 1000.0, 12.0, 1.0);
        band.enabled = true;
        eq.add_band(band);

        let frames = 4800;
        let data = generate_sine(1000.0, 48000.0, frames, 0.3);

        // Process with band enabled
        let mut buf1 = AudioBuffer::from_interleaved(data.clone(), 1);
        eq.process(&mut buf1);
        let rms_enabled = rms_of_buffer(&buf1, 0, frames);

        // Disable and reset
        eq.bands[0].enabled = false;
        eq.reset();
        let mut buf2 = AudioBuffer::from_interleaved(data.clone(), 1);
        eq.process(&mut buf2);
        let rms_disabled = rms_of_buffer(&buf2, 0, frames);

        assert!(
            rms_enabled > rms_disabled * 1.5,
            "Enabled band should boost more than disabled: enabled={rms_enabled}, disabled={rms_disabled}"
        );

        // Re-verify disabled is close to original
        let rms_orig = rms_of_buffer(&AudioBuffer::from_interleaved(data, 1), 0, frames);
        assert!(
            (rms_disabled - rms_orig).abs() < 0.01,
            "Disabled band should pass through: disabled={rms_disabled}, orig={rms_orig}"
        );
    }

    #[test]
    fn test_set_sample_rate_updates_coefficients() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::LowPass, 1000.0, 0.0, 0.707));

        // Store coefficients at 48kHz
        let b0_48k = eq.bands[0].b0;

        // Change to 96kHz — coefficients should change
        eq.set_sample_rate(96000.0);
        let b0_96k = eq.bands[0].b0;

        assert!(
            (b0_48k - b0_96k).abs() > 1e-6,
            "Coefficients should change when sample rate changes: b0@48k={b0_48k}, b0@96k={b0_96k}"
        );
    }

    #[test]
    fn test_multiple_bands_active() {
        let mut eq = ParametricEq::new(48000.0);
        // Low shelf boost + high shelf boost
        eq.add_band(EqBand::new(FilterType::LowShelf, 300.0, 6.0, 0.707));
        eq.add_band(EqBand::new(FilterType::HighShelf, 5000.0, 6.0, 0.707));

        let frames = 4800;
        // Test that both a low and high frequency get boosted
        let data_low = generate_sine(100.0, 48000.0, frames, 0.3);
        let mut buf_low = AudioBuffer::from_interleaved(data_low, 1);
        let rms_before_low = rms_of_buffer(&buf_low, 0, frames);
        eq.process(&mut buf_low);
        let rms_after_low = rms_of_buffer(&buf_low, 0, frames);

        eq.reset();
        let data_high = generate_sine(10000.0, 48000.0, frames, 0.3);
        let mut buf_high = AudioBuffer::from_interleaved(data_high, 1);
        let rms_before_high = rms_of_buffer(&buf_high, 0, frames);
        eq.process(&mut buf_high);
        let rms_after_high = rms_of_buffer(&buf_high, 0, frames);

        assert!(
            rms_after_low > rms_before_low * 1.3,
            "Low freq should be boosted by low shelf"
        );
        assert!(
            rms_after_high > rms_before_high * 1.3,
            "High freq should be boosted by high shelf"
        );
    }

    #[test]
    fn test_lowpass_passes_low_frequencies() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::LowPass, 5000.0, 0.0, 0.707));

        let frames = 4800;
        let data = generate_sine(100.0, 48000.0, frames, 0.5);
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        let rms_before = rms_of_buffer(&buf, 0, frames);
        eq.process(&mut buf);
        let rms_after = rms_of_buffer(&buf, 0, frames);

        // Low frequency should pass through mostly unchanged
        assert!(
            (rms_after / rms_before - 1.0).abs() < 0.1,
            "LowPass at 5kHz should pass 100Hz: before={rms_before}, after={rms_after}"
        );
    }

    #[test]
    fn test_highpass_passes_high_frequencies() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::HighPass, 500.0, 0.0, 0.707));

        let frames = 4800;
        let data = generate_sine(10000.0, 48000.0, frames, 0.5);
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        let rms_before = rms_of_buffer(&buf, 0, frames);
        eq.process(&mut buf);
        let rms_after = rms_of_buffer(&buf, 0, frames);

        assert!(
            (rms_after / rms_before - 1.0).abs() < 0.1,
            "HighPass at 500Hz should pass 10kHz: before={rms_before}, after={rms_after}"
        );
    }

    #[test]
    fn test_peak_cut_reduces_target_frequency() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::Peak, 1000.0, -12.0, 1.0));

        let frames = 4800;
        let data = generate_sine(1000.0, 48000.0, frames, 0.5);
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        let rms_before = rms_of_buffer(&buf, 0, frames);
        eq.process(&mut buf);
        let rms_after = rms_of_buffer(&buf, 0, frames);

        assert!(
            rms_after < rms_before * 0.5,
            "Peak cut at 1kHz should reduce 1kHz signal: before={rms_before}, after={rms_after}"
        );
    }

    #[test]
    fn test_low_shelf_leaves_high_frequencies_unchanged() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::LowShelf, 300.0, 12.0, 0.707));

        let frames = 4800;
        let data = generate_sine(10000.0, 48000.0, frames, 0.3);
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        let rms_before = rms_of_buffer(&buf, 0, frames);
        eq.process(&mut buf);
        let rms_after = rms_of_buffer(&buf, 0, frames);

        // High frequency far above shelf should be mostly unaffected
        assert!(
            (rms_after / rms_before - 1.0).abs() < 0.3,
            "LowShelf at 300Hz should not significantly change 10kHz: ratio={}",
            rms_after / rms_before
        );
    }

    #[test]
    fn test_eq_reset_clears_filter_state() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::Peak, 1000.0, 12.0, 1.0));

        // Process some audio to build up state
        let frames = 480;
        let data = generate_sine(1000.0, 48000.0, frames, 0.5);
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        eq.process(&mut buf);

        // Reset
        eq.reset();

        // Verify state is zeroed
        for band in &eq.bands {
            assert_eq!(band.state[0][0], 0.0);
            assert_eq!(band.state[0][1], 0.0);
            assert_eq!(band.state[1][0], 0.0);
            assert_eq!(band.state[1][1], 0.0);
        }
    }

    #[test]
    fn test_three_band_eq() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(EqBand::new(FilterType::LowShelf, 200.0, 6.0, 0.707));
        eq.add_band(EqBand::new(FilterType::Peak, 1000.0, -6.0, 1.0));
        eq.add_band(EqBand::new(FilterType::HighShelf, 8000.0, 3.0, 0.707));
        assert_eq!(eq.bands.len(), 3);

        // Just verify it processes without panicking or producing NaN
        let frames = 4800;
        let data = generate_sine(440.0, 48000.0, frames, 0.5);
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        eq.process(&mut buf);

        for i in 0..frames {
            assert!(
                buf.get(i as u32, 0).is_finite(),
                "Output should be finite at frame {i}"
            );
        }
    }
}
