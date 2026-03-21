//! De-esser — sibilance reduction backed by dhvani.

use crate::buffer::AudioBuffer;
use crate::constants::{
    DEFAULT_DEESSER_FREQ_HZ, DEFAULT_DEESSER_Q, DEFAULT_DEESSER_REDUCTION_DB,
    DEFAULT_DEESSER_THRESHOLD_DB,
};

/// Sibilance reduction processor.
#[derive(Debug, Clone)]
pub struct DeEsser {
    /// Center frequency of the sibilance band in Hz.
    pub freq_hz: f32,
    /// Threshold in dB — sibilance above this is reduced.
    pub threshold_db: f32,
    /// Maximum gain reduction in dB.
    pub reduction_db: f32,
    /// Q factor / bandwidth.
    pub q: f32,
    sample_rate: f32,
    channels: u16,
    inner: dhvani::dsp::DeEsser,
}

impl DeEsser {
    pub fn new(sample_rate: f32) -> Self {
        Self::with_channels(sample_rate, 2)
    }

    pub fn with_channels(sample_rate: f32, channels: u16) -> Self {
        let params = dhvani::dsp::DeEsserParams {
            freq_hz: DEFAULT_DEESSER_FREQ_HZ,
            threshold_db: DEFAULT_DEESSER_THRESHOLD_DB,
            reduction_db: DEFAULT_DEESSER_REDUCTION_DB,
            q: DEFAULT_DEESSER_Q,
        };
        let mut s = Self {
            freq_hz: DEFAULT_DEESSER_FREQ_HZ,
            threshold_db: DEFAULT_DEESSER_THRESHOLD_DB,
            reduction_db: DEFAULT_DEESSER_REDUCTION_DB,
            q: DEFAULT_DEESSER_Q,
            sample_rate,
            channels,
            inner: dhvani::dsp::DeEsser::new(params, sample_rate as u32, channels as u32),
        };
        s.sync_inner();
        s
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.sync_inner();
    }

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

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    fn sync_inner(&mut self) {
        let params = dhvani::dsp::DeEsserParams {
            freq_hz: self.freq_hz,
            threshold_db: self.threshold_db,
            reduction_db: self.reduction_db,
            q: self.q.max(0.01),
        };
        self.inner =
            dhvani::dsp::DeEsser::new(params, self.sample_rate as u32, self.channels as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_input() {
        let mut de = DeEsser::new(48000.0);
        let mut buf = AudioBuffer::new(2, 256);
        de.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| *s == 0.0));
    }

    #[test]
    fn sibilant_signal_processed() {
        let sr = 48000.0;
        let mut de = DeEsser::new(sr);
        de.threshold_db = -40.0; // very sensitive
        de.reduction_db = 12.0;
        // Generate a 7kHz sine (sibilance range)
        let n = 4096;
        let mut buf = AudioBuffer::new(1, n as u32);
        for i in 0..n {
            let s = (2.0 * std::f32::consts::PI * 7000.0 * i as f32 / sr).sin() * 0.8;
            buf.set(i as u32, 0, s);
        }
        de.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn output_finite() {
        let mut de = DeEsser::new(48000.0);
        let samples: Vec<f32> = (0..512).map(|i| (i as f32 * 0.1).sin()).collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 2);
        de.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }
}
