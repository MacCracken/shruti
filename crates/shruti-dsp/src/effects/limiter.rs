//! Brickwall limiter — backed by dhvani.

use crate::buffer::AudioBuffer;
use crate::constants::{DEFAULT_LIMITER_CEILING, DEFAULT_LIMITER_RELEASE};

/// Brickwall limiter — prevents signal from exceeding the ceiling.
#[derive(Debug, Clone)]
pub struct Limiter {
    /// Ceiling in dB (maximum output level).
    pub ceiling_db: f32,
    /// Release time in seconds.
    pub release: f32,
    sample_rate: f32,
    inner: dhvani::dsp::EnvelopeLimiter,
}

impl Limiter {
    pub fn new(sample_rate: f32) -> Self {
        let params = dhvani::dsp::LimiterParams {
            ceiling_db: DEFAULT_LIMITER_CEILING,
            release_ms: DEFAULT_LIMITER_RELEASE * 1000.0,
            knee_db: 0.0,
        };
        Self {
            ceiling_db: DEFAULT_LIMITER_CEILING,
            release: DEFAULT_LIMITER_RELEASE,
            sample_rate,
            inner: dhvani::dsp::EnvelopeLimiter::new(params, sample_rate as u32),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.rebuild();
    }

    /// Process an audio buffer in place.
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        self.rebuild();
        if let Ok(mut dbuf) = dhvani::buffer::AudioBuffer::from_interleaved(
            buffer.as_interleaved().to_vec(),
            buffer.channels() as u32,
            self.sample_rate as u32,
        ) {
            self.inner.process(&mut dbuf);
            buffer.as_interleaved_mut().copy_from_slice(&dbuf.samples);
        }
    }

    /// Reset limiter state.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    fn rebuild(&mut self) {
        let params = dhvani::dsp::LimiterParams {
            ceiling_db: self.ceiling_db,
            release_ms: self.release * 1000.0,
            knee_db: 0.0,
        };
        self.inner = dhvani::dsp::EnvelopeLimiter::new(params, self.sample_rate as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_input() {
        let mut lim = Limiter::new(48000.0);
        let mut buf = AudioBuffer::new(1, 256);
        lim.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| *s == 0.0));
    }

    #[test]
    fn limits_peaks() {
        let mut lim = Limiter::new(48000.0);
        lim.ceiling_db = -6.0;
        let ceiling_linear = 10.0f32.powf(-6.0 / 20.0);
        let mut buf = AudioBuffer::from_interleaved(vec![1.0, -1.0, 0.9, -0.9], 1);
        lim.process(&mut buf);
        for &s in buf.as_interleaved() {
            assert!(
                s.abs() <= ceiling_linear + 0.05,
                "sample {} exceeds ceiling {}",
                s,
                ceiling_linear
            );
        }
    }

    #[test]
    fn output_finite() {
        let mut lim = Limiter::new(48000.0);
        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 1);
        lim.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }
}
