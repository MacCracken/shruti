//! Parametric EQ with biquad filters — backed by dhvani.

use crate::buffer::AudioBuffer;

/// Filter type for a single EQ band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    LowShelf,
    HighShelf,
    Peak,
    LowPass,
    HighPass,
}

/// A single EQ band configuration.
#[derive(Debug, Clone)]
pub struct EqBand {
    pub filter_type: FilterType,
    pub frequency: f32,
    pub gain_db: f32,
    pub q: f32,
    pub enabled: bool,
}

impl EqBand {
    pub fn new(filter_type: FilterType, frequency: f32, gain_db: f32, q: f32) -> Self {
        Self {
            filter_type,
            frequency,
            gain_db,
            q,
            enabled: true,
        }
    }

    fn to_dhvani_config(&self) -> dhvani::dsp::EqBandConfig {
        let band_type = match self.filter_type {
            FilterType::LowShelf => dhvani::dsp::BandType::LowShelf,
            FilterType::HighShelf => dhvani::dsp::BandType::HighShelf,
            FilterType::Peak => dhvani::dsp::BandType::Peaking,
            FilterType::LowPass => dhvani::dsp::BandType::LowPass,
            FilterType::HighPass => dhvani::dsp::BandType::HighPass,
        };
        dhvani::dsp::EqBandConfig {
            band_type,
            freq_hz: self.frequency,
            gain_db: self.gain_db,
            q: self.q,
            enabled: self.enabled,
        }
    }
}

/// Multi-band parametric EQ.
#[derive(Debug, Clone)]
pub struct ParametricEq {
    bands: Vec<EqBand>,
    sample_rate: f32,
    channels: u16,
    inner: dhvani::dsp::ParametricEq,
}

impl ParametricEq {
    pub fn new(bands: Vec<EqBand>, sample_rate: f32, channels: u16) -> Self {
        let dhvani_bands: Vec<dhvani::dsp::EqBandConfig> =
            bands.iter().map(|b| b.to_dhvani_config()).collect();
        let inner =
            dhvani::dsp::ParametricEq::new(dhvani_bands, sample_rate as u32, channels as u32);
        Self {
            bands,
            sample_rate,
            channels,
            inner,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.rebuild();
    }

    /// Update a band's parameters and rebuild coefficients.
    pub fn set_band(&mut self, index: usize, band: EqBand) {
        if index < self.bands.len() {
            self.bands[index] = band;
            self.rebuild();
        }
    }

    /// Get the current bands.
    pub fn bands(&self) -> &[EqBand] {
        &self.bands
    }

    /// Process an audio buffer through the EQ.
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        if self
            .bands
            .iter()
            .all(|b| !b.enabled || b.gain_db.abs() < 0.01)
        {
            return;
        }

        if let Ok(mut dbuf) = dhvani::buffer::AudioBuffer::from_interleaved(
            buffer.as_interleaved().to_vec(),
            buffer.channels() as u32,
            self.sample_rate as u32,
        ) {
            self.inner.process(&mut dbuf);
            buffer.as_interleaved_mut().copy_from_slice(&dbuf.samples);
        }
    }

    /// Reset filter state.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    fn rebuild(&mut self) {
        let dhvani_bands: Vec<dhvani::dsp::EqBandConfig> =
            self.bands.iter().map(|b| b.to_dhvani_config()).collect();
        self.inner = dhvani::dsp::ParametricEq::new(
            dhvani_bands,
            self.sample_rate as u32,
            self.channels as u32,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(freq: f32, sr: f32, n: usize) -> AudioBuffer {
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin())
            .collect();
        AudioBuffer::from_interleaved(samples, 1)
    }

    fn rms(buf: &AudioBuffer) -> f32 {
        let sum: f64 = buf
            .as_interleaved()
            .iter()
            .map(|s| (*s as f64).powi(2))
            .sum();
        (sum / buf.as_interleaved().len() as f64).sqrt() as f32
    }

    #[test]
    fn flat_eq_passthrough() {
        let bands = vec![EqBand::new(FilterType::Peak, 1000.0, 0.0, 1.0)];
        let mut eq = ParametricEq::new(bands, 48000.0, 1);
        let original: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(original.clone(), 1);
        eq.process(&mut buf);
        assert_eq!(buf.as_interleaved(), &original[..]);
    }

    #[test]
    fn boost_increases_energy() {
        let bands = vec![EqBand::new(FilterType::Peak, 1000.0, 12.0, 1.0)];
        let mut eq = ParametricEq::new(bands, 48000.0, 1);
        let mut buf = make_sine(1000.0, 48000.0, 4096);
        let before = rms(&buf);
        eq.process(&mut buf);
        let after = rms(&buf);
        assert!(after > before, "boost: {before} -> {after}");
    }

    #[test]
    fn cut_decreases_energy() {
        let bands = vec![EqBand::new(FilterType::Peak, 1000.0, -12.0, 1.0)];
        let mut eq = ParametricEq::new(bands, 48000.0, 1);
        let mut buf = make_sine(1000.0, 48000.0, 4096);
        let before = rms(&buf);
        eq.process(&mut buf);
        let after = rms(&buf);
        assert!(after < before, "cut: {before} -> {after}");
    }

    #[test]
    fn disabled_band_passthrough() {
        let mut band = EqBand::new(FilterType::Peak, 1000.0, 12.0, 1.0);
        band.enabled = false;
        let bands = vec![band];
        let mut eq = ParametricEq::new(bands, 48000.0, 1);
        let original: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(original.clone(), 1);
        eq.process(&mut buf);
        assert_eq!(buf.as_interleaved(), &original[..]);
    }

    #[test]
    fn output_finite() {
        let bands = vec![
            EqBand::new(FilterType::LowShelf, 200.0, 6.0, 0.707),
            EqBand::new(FilterType::Peak, 1000.0, -3.0, 1.0),
            EqBand::new(FilterType::HighShelf, 5000.0, 4.0, 0.707),
        ];
        let mut eq = ParametricEq::new(bands, 48000.0, 2);
        let samples: Vec<f32> = (0..8192)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / 48000.0).sin() * 0.5)
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 2);
        eq.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }
}
