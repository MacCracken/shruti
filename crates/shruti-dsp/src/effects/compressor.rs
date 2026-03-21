//! Dynamic range compressor — backed by dhvani.

use crate::buffer::AudioBuffer;
use crate::constants::{
    DEFAULT_COMPRESSOR_ATTACK, DEFAULT_COMPRESSOR_KNEE, DEFAULT_COMPRESSOR_RATIO,
    DEFAULT_COMPRESSOR_RELEASE, DEFAULT_COMPRESSOR_THRESHOLD,
};

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
    inner: dhvani::dsp::Compressor,
}

impl Compressor {
    pub fn new(sample_rate: f32) -> Self {
        let mut c = Self {
            threshold_db: DEFAULT_COMPRESSOR_THRESHOLD,
            ratio: DEFAULT_COMPRESSOR_RATIO,
            attack: DEFAULT_COMPRESSOR_ATTACK,
            release: DEFAULT_COMPRESSOR_RELEASE,
            makeup_db: 0.0,
            knee_db: DEFAULT_COMPRESSOR_KNEE,
            sample_rate,
            inner: dhvani::dsp::Compressor::new(
                dhvani::dsp::CompressorParams::default(),
                sample_rate as u32,
            ),
        };
        c.sync_inner();
        c
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.sync_inner();
    }

    /// Process an audio buffer in place.
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        self.sync_inner();
        if let Ok(mut dbuf) = dhvani::buffer::AudioBuffer::from_interleaved(
            buffer.as_interleaved().to_vec(),
            buffer.channels() as u32,
            self.sample_rate as u32,
        ) {
            self.inner.process(&mut dbuf);
            buffer.as_interleaved_mut().copy_from_slice(&dbuf.samples);
        }
    }

    fn sync_inner(&mut self) {
        let params = dhvani::dsp::CompressorParams {
            threshold_db: self.threshold_db,
            ratio: self.ratio.max(1.0),
            attack_ms: self.attack * 1000.0,
            release_ms: self.release * 1000.0,
            makeup_gain_db: self.makeup_db,
            knee_db: self.knee_db,
        };
        self.inner = dhvani::dsp::Compressor::new(params, self.sample_rate as u32);
    }
}

#[inline]
pub fn db_to_linear(db: f32) -> f32 {
    dhvani::dsp::db_to_amplitude(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(sr: f32, n: usize, amp: f32) -> AudioBuffer {
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr).sin() * amp)
            .collect();
        AudioBuffer::from_interleaved(samples, 1)
    }

    #[test]
    fn silent_input() {
        let mut comp = Compressor::new(48000.0);
        let mut buf = AudioBuffer::new(1, 256);
        comp.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| *s == 0.0));
    }

    #[test]
    fn loud_signal_compressed() {
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 8.0;
        comp.attack = 0.001;
        comp.release = 0.01;
        let mut buf = make_sine(48000.0, 48000, 0.9);
        let before_peak = buf
            .as_interleaved()
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        comp.process(&mut buf);
        let after_peak = buf
            .as_interleaved()
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        assert!(
            after_peak < before_peak,
            "compression should reduce peak: {before_peak} -> {after_peak}"
        );
    }

    #[test]
    fn output_finite() {
        let mut comp = Compressor::new(48000.0);
        let mut buf = make_sine(48000.0, 4096, 1.0);
        comp.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }
}
