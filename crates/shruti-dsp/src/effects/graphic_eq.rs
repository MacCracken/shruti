//! 10-band ISO graphic equalizer — backed by dhvani.

use crate::buffer::AudioBuffer;

pub use dhvani::dsp::{GraphicEqSettings, ISO_BANDS};

/// 10-band ISO graphic equalizer with named presets.
pub struct GraphicEq {
    sample_rate: f32,
    channels: u16,
    inner: dhvani::dsp::GraphicEq,
}

impl std::fmt::Debug for GraphicEq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphicEq")
            .field("sample_rate", &self.sample_rate)
            .field("channels", &self.channels)
            .finish()
    }
}

impl GraphicEq {
    pub fn new(sample_rate: f32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
            inner: dhvani::dsp::GraphicEq::new(sample_rate as u32, channels as u32),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let settings = self.inner.settings().clone();
        self.inner = dhvani::dsp::GraphicEq::new(sample_rate as u32, self.channels as u32);
        self.inner.set_settings(settings);
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

    pub fn load_preset(&mut self, name: &str) {
        self.inner.load_preset(name);
    }

    pub fn set_band(&mut self, index: usize, gain_db: f32) {
        self.inner.set_band(index, gain_db);
    }

    pub fn set_settings(&mut self, settings: GraphicEqSettings) {
        self.inner.set_settings(settings);
    }

    pub fn settings(&self) -> &GraphicEqSettings {
        self.inner.settings()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.inner.set_enabled(enabled);
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_input() {
        let mut eq = GraphicEq::new(48000.0, 1);
        eq.set_enabled(true);
        eq.load_preset("rock");
        let mut buf = AudioBuffer::new(1, 256);
        eq.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| *s == 0.0));
    }

    #[test]
    fn flat_eq_passthrough() {
        let mut eq = GraphicEq::new(48000.0, 1);
        // Flat + disabled = passthrough
        let samples: Vec<f32> = (0..256).map(|i| (i as f32 * 0.05).sin()).collect();
        let original = samples.clone();
        let mut buf = AudioBuffer::from_interleaved(samples, 1);
        eq.process(&mut buf);
        assert_eq!(buf.as_interleaved(), &original);
    }

    #[test]
    fn preset_modifies_signal() {
        let mut eq = GraphicEq::new(48000.0, 1);
        eq.load_preset("bass");
        eq.set_enabled(true);

        let n = 4096;
        let mut buf = AudioBuffer::new(1, n as u32);
        for i in 0..n {
            let s = (2.0 * std::f32::consts::PI * 100.0 * i as f32 / 48000.0).sin();
            buf.set(i as u32, 0, s);
        }
        let before: Vec<f32> = buf.as_interleaved().to_vec();
        eq.process(&mut buf);
        assert_ne!(
            buf.as_interleaved(),
            &before,
            "bass preset should modify low-freq signal"
        );
    }

    #[test]
    fn output_finite() {
        let mut eq = GraphicEq::new(48000.0, 2);
        eq.load_preset("electronic");
        eq.set_enabled(true);
        let samples: Vec<f32> = (0..512).map(|i| (i as f32 * 0.01).sin()).collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 2);
        eq.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }
}
