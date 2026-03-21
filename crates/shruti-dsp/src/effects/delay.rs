//! Stereo delay effect — backed by dhvani.

use crate::buffer::AudioBuffer;
use crate::constants::{
    DEFAULT_DELAY_FEEDBACK, DEFAULT_DELAY_MIX, DEFAULT_DELAY_TIME, MAX_DELAY_SECONDS,
};

/// Stereo delay effect with feedback and dry/wet mix.
#[derive(Debug, Clone)]
pub struct Delay {
    /// Delay time in seconds.
    pub time: f32,
    /// Feedback amount (0.0 to <1.0).
    pub feedback: f32,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    pub mix: f32,
    sample_rate: f32,
    inner: dhvani::dsp::DelayLine,
}

impl Delay {
    pub fn new(sample_rate: f32) -> Self {
        let time_ms = DEFAULT_DELAY_TIME * 1000.0;
        let max_ms = MAX_DELAY_SECONDS * 1000.0;
        Self {
            time: DEFAULT_DELAY_TIME,
            feedback: DEFAULT_DELAY_FEEDBACK,
            mix: DEFAULT_DELAY_MIX,
            sample_rate,
            inner: dhvani::dsp::DelayLine::new(
                time_ms,
                max_ms,
                DEFAULT_DELAY_FEEDBACK,
                DEFAULT_DELAY_MIX,
                sample_rate as u32,
                2, // stereo
            ),
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

    /// Reset internal delay buffer.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    fn rebuild(&mut self) {
        let time_ms = (self.time * 1000.0).clamp(0.0, MAX_DELAY_SECONDS * 1000.0);
        let max_ms = MAX_DELAY_SECONDS * 1000.0;
        self.inner = dhvani::dsp::DelayLine::new(
            time_ms,
            max_ms,
            self.feedback.clamp(0.0, 0.99),
            self.mix.clamp(0.0, 1.0),
            self.sample_rate as u32,
            2,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_input() {
        let mut delay = Delay::new(48000.0);
        let mut buf = AudioBuffer::new(2, 256);
        delay.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| *s == 0.0));
    }

    #[test]
    fn delay_modifies_signal() {
        let mut delay = Delay::new(48000.0);
        delay.time = 0.01; // 10ms
        delay.mix = 0.5;
        let samples: Vec<f32> = (0..9600)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / 48000.0).sin() * 0.5)
            .collect();
        let original = samples.clone();
        let mut buf = AudioBuffer::from_interleaved(samples, 2);
        delay.process(&mut buf);
        // With delay + wet mix, signal should differ
        let any_different = buf
            .as_interleaved()
            .iter()
            .zip(original.iter())
            .any(|(a, b)| (a - b).abs() > 1e-6);
        assert!(any_different, "delay should modify the signal");
    }

    #[test]
    fn output_finite() {
        let mut delay = Delay::new(48000.0);
        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 1);
        delay.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }
}
