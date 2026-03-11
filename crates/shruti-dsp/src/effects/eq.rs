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
}
