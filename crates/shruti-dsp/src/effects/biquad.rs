//! Biquad filter — backed by dhvani.

use crate::buffer::AudioBuffer;

pub use dhvani::dsp::{BiquadCoeffs, FilterType};

/// Biquad filter with configurable type, frequency, and Q.
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    pub filter_type: FilterType,
    pub freq_hz: f32,
    pub q: f32,
    sample_rate: f32,
    channels: u16,
    inner: dhvani::dsp::BiquadFilter,
}

impl BiquadFilter {
    pub fn new(
        filter_type: FilterType,
        freq_hz: f32,
        q: f32,
        sample_rate: f32,
        channels: u16,
    ) -> Self {
        Self {
            filter_type,
            freq_hz,
            q,
            sample_rate,
            channels,
            inner: dhvani::dsp::BiquadFilter::new(
                filter_type,
                freq_hz,
                q,
                sample_rate as u32,
                channels as u32,
            ),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.rebuild();
    }

    pub fn set_params(&mut self, filter_type: FilterType, freq_hz: f32, q: f32) {
        self.filter_type = filter_type;
        self.freq_hz = freq_hz;
        self.q = q;
        self.inner.set_params(filter_type, freq_hz, q);
    }

    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        if let Ok(mut dbuf) = dhvani::buffer::AudioBuffer::from_interleaved(
            buffer.as_interleaved().to_vec(),
            buffer.channels() as u32,
            self.sample_rate as u32,
        ) {
            self.inner.process(&mut dbuf);
            buffer.as_interleaved_mut().copy_from_slice(&dbuf.samples);
        }
    }

    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        self.inner.process_sample(sample, channel)
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    fn rebuild(&mut self) {
        self.inner = dhvani::dsp::BiquadFilter::new(
            self.filter_type,
            self.freq_hz,
            self.q,
            self.sample_rate as u32,
            self.channels as u32,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_input() {
        let mut filter = BiquadFilter::new(FilterType::LowPass, 1000.0, 0.707, 48000.0, 1);
        let mut buf = AudioBuffer::new(1, 256);
        filter.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| *s == 0.0));
    }

    #[test]
    fn lowpass_attenuates_high_freq() {
        let sr = 48000.0;
        let mut filter = BiquadFilter::new(FilterType::LowPass, 200.0, 0.707, sr, 1);
        // Generate a high-frequency sine (10kHz)
        let n = 4096;
        let mut buf = AudioBuffer::new(1, n as u32);
        for i in 0..n {
            let s = (2.0 * std::f32::consts::PI * 10000.0 * i as f32 / sr).sin();
            buf.set(i as u32, 0, s);
        }
        let before_rms: f32 =
            (buf.as_interleaved().iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt();
        filter.process(&mut buf);
        let after_rms: f32 =
            (buf.as_interleaved().iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt();
        assert!(
            after_rms < before_rms * 0.1,
            "lowpass should attenuate 10kHz signal"
        );
    }

    #[test]
    fn output_finite() {
        let mut filter = BiquadFilter::new(FilterType::HighPass, 500.0, 1.0, 48000.0, 2);
        let samples: Vec<f32> = (0..512).map(|i| (i as f32 * 0.01).sin()).collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 2);
        filter.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn process_sample_works() {
        let mut filter = BiquadFilter::new(FilterType::LowPass, 5000.0, 0.707, 48000.0, 1);
        let out = filter.process_sample(1.0, 0);
        assert!(out.is_finite());
    }
}
