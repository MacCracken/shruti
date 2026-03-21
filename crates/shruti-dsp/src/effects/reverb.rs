//! Schroeder-style reverb — backed by dhvani.

use crate::buffer::AudioBuffer;
use crate::constants::{DEFAULT_REVERB_DAMPING, DEFAULT_REVERB_MIX, DEFAULT_REVERB_ROOM_SIZE};

/// Schroeder-style reverb with comb and allpass filters.
#[derive(Debug, Clone)]
pub struct Reverb {
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    pub mix: f32,
    /// Room size / decay (0.0 to 1.0).
    pub room_size: f32,
    /// High frequency damping (0.0 to 1.0).
    pub damping: f32,
    sample_rate: f32,
    inner: dhvani::dsp::Reverb,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let params = dhvani::dsp::ReverbParams {
            room_size: DEFAULT_REVERB_ROOM_SIZE,
            damping: DEFAULT_REVERB_DAMPING,
            mix: DEFAULT_REVERB_MIX,
        };
        Self {
            mix: DEFAULT_REVERB_MIX,
            room_size: DEFAULT_REVERB_ROOM_SIZE,
            damping: DEFAULT_REVERB_DAMPING,
            sample_rate,
            inner: dhvani::dsp::Reverb::new(params, sample_rate as u32),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.rebuild();
    }

    /// Process an audio buffer in place.
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        self.sync_params();
        if let Ok(mut dbuf) = dhvani::buffer::AudioBuffer::from_interleaved(
            buffer.as_interleaved().to_vec(),
            buffer.channels() as u32,
            self.sample_rate as u32,
        ) {
            self.inner.process(&mut dbuf);
            buffer.as_interleaved_mut().copy_from_slice(&dbuf.samples);
        }
    }

    /// Reset internal state (call on seek/track change).
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Recompute internal parameters (backward compatibility — now automatic).
    pub fn update_parameters(&mut self) {
        self.sync_params();
    }

    fn sync_params(&mut self) {
        let params = dhvani::dsp::ReverbParams {
            room_size: self.room_size.clamp(0.0, 1.0),
            damping: self.damping.clamp(0.0, 1.0),
            mix: self.mix.clamp(0.0, 1.0),
        };
        self.inner.set_params(params);
    }

    fn rebuild(&mut self) {
        let params = dhvani::dsp::ReverbParams {
            room_size: self.room_size,
            damping: self.damping,
            mix: self.mix,
        };
        self.inner = dhvani::dsp::Reverb::new(params, self.sample_rate as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_input() {
        let mut rev = Reverb::new(48000.0);
        let mut buf = AudioBuffer::new(1, 256);
        rev.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| *s == 0.0));
    }

    #[test]
    fn zero_wet_passthrough() {
        let mut rev = Reverb::new(48000.0);
        rev.mix = 0.0;
        let original: Vec<f32> = (0..4800)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(original.clone(), 1);
        rev.process(&mut buf);
        assert_eq!(buf.as_interleaved(), &original[..]);
    }

    #[test]
    fn impulse_produces_tail() {
        let mut rev = Reverb::new(48000.0);
        rev.mix = 1.0;
        rev.room_size = 0.8;
        let mut samples = vec![0.0f32; 48000];
        samples[0] = 1.0;
        let mut buf = AudioBuffer::from_interleaved(samples, 1);
        rev.process(&mut buf);
        let tail_energy: f64 = buf.as_interleaved()[1000..]
            .iter()
            .map(|s| (*s as f64).powi(2))
            .sum();
        assert!(tail_energy > 0.0, "reverb should produce a tail");
    }

    #[test]
    fn output_finite() {
        let mut rev = Reverb::new(48000.0);
        let samples: Vec<f32> = (0..4800)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 1);
        rev.process(&mut buf);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }
}
