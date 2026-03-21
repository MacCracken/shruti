//! Modulated delay (chorus/flanger) — backed by dhvani.

use crate::buffer::AudioBuffer;
use crate::constants::{
    DEFAULT_MOD_DELAY_BASE_MS, DEFAULT_MOD_DELAY_DEPTH_MS, DEFAULT_MOD_DELAY_FEEDBACK,
    DEFAULT_MOD_DELAY_MIX, DEFAULT_MOD_DELAY_RATE_HZ,
};

pub use dhvani::dsp::ModulatedDelayParams;

/// Modulated delay producing chorus or flanger effects.
#[derive(Debug, Clone)]
pub struct ModulatedDelay {
    pub params: ModulatedDelayParams,
    sample_rate: f32,
    channels: u16,
    inner: dhvani::dsp::ModulatedDelay,
}

impl ModulatedDelay {
    pub fn new(params: ModulatedDelayParams, sample_rate: f32, channels: u16) -> Self {
        Self {
            inner: dhvani::dsp::ModulatedDelay::new(
                params.clone(),
                sample_rate as u32,
                channels as u32,
            ),
            params,
            sample_rate,
            channels,
        }
    }

    /// Create with default chorus-like parameters.
    pub fn default_chorus(sample_rate: f32, channels: u16) -> Self {
        Self::new(
            ModulatedDelayParams {
                base_delay_ms: DEFAULT_MOD_DELAY_BASE_MS,
                depth_ms: DEFAULT_MOD_DELAY_DEPTH_MS,
                rate_hz: DEFAULT_MOD_DELAY_RATE_HZ,
                feedback: DEFAULT_MOD_DELAY_FEEDBACK,
                mix: DEFAULT_MOD_DELAY_MIX,
            },
            sample_rate,
            channels,
        )
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.rebuild();
    }

    pub fn set_params(&mut self, params: ModulatedDelayParams) {
        self.params = params;
        self.rebuild();
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

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    fn rebuild(&mut self) {
        self.inner = dhvani::dsp::ModulatedDelay::new(
            self.params.clone(),
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
        let mut md = ModulatedDelay::default_chorus(48000.0, 2);
        let mut buf = AudioBuffer::new(2, 256);
        md.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| *s == 0.0));
    }

    #[test]
    fn chorus_modifies_signal() {
        let mut md = ModulatedDelay::default_chorus(48000.0, 1);
        let n = 4096;
        let mut buf = AudioBuffer::new(1, n as u32);
        for i in 0..n {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin() * 0.5;
            buf.set(i as u32, 0, s);
        }
        let before: Vec<f32> = buf.as_interleaved().to_vec();
        md.process(&mut buf);
        assert_ne!(buf.as_interleaved(), &before, "chorus should modify signal");
    }

    #[test]
    fn output_finite() {
        let mut md = ModulatedDelay::default_chorus(48000.0, 2);
        let samples: Vec<f32> = (0..512).map(|i| (i as f32 * 0.01).sin()).collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 2);
        md.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }
}
