//! LFO — low-frequency oscillator for modulation, backed by dhvani.

use crate::constants::{DEFAULT_LFO_DEPTH, DEFAULT_LFO_RATE};

pub use dhvani::dsp::LfoShape;

/// Low-frequency oscillator for parameter modulation.
#[derive(Debug, Clone)]
pub struct Lfo {
    sample_rate: f32,
    inner: dhvani::dsp::Lfo,
}

impl Lfo {
    pub fn new(shape: LfoShape, rate: f32, depth: f32, sample_rate: f32) -> Self {
        Self {
            sample_rate,
            inner: dhvani::dsp::Lfo::new(shape, rate, depth, sample_rate as u32),
        }
    }

    /// Create with default parameters (1 Hz, 0.5 depth, sine).
    pub fn default_sine(sample_rate: f32) -> Self {
        Self::new(
            LfoShape::Sine,
            DEFAULT_LFO_RATE,
            DEFAULT_LFO_DEPTH,
            sample_rate,
        )
    }

    /// Advance one sample and return the current value (-depth to +depth).
    pub fn tick(&mut self) -> f32 {
        self.inner.tick()
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.inner.set_rate(rate);
    }

    pub fn set_depth(&mut self, depth: f32) {
        self.inner.set_depth(depth);
    }

    pub fn set_shape(&mut self, shape: LfoShape) {
        self.inner.set_shape(shape);
    }

    pub fn rate(&self) -> f32 {
        self.inner.rate()
    }

    pub fn depth(&self) -> f32 {
        self.inner.depth()
    }

    pub fn shape(&self) -> LfoShape {
        self.inner.shape()
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let shape = self.inner.shape();
        let rate = self.inner.rate();
        let depth = self.inner.depth();
        self.inner = dhvani::dsp::Lfo::new(shape, rate, depth, sample_rate as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_lfo_oscillates() {
        let mut lfo = Lfo::new(LfoShape::Sine, 10.0, 1.0, 48000.0);
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for _ in 0..4800 {
            let v = lfo.tick();
            min = min.min(v);
            max = max.max(v);
        }
        assert!(max > 0.5, "max should be positive, got {max}");
        assert!(min < -0.5, "min should be negative, got {min}");
    }

    #[test]
    fn output_in_depth_range() {
        let depth = 0.3;
        let mut lfo = Lfo::new(LfoShape::Triangle, 5.0, depth, 48000.0);
        for _ in 0..9600 {
            let v = lfo.tick();
            assert!(
                v >= -depth - 0.01 && v <= depth + 0.01,
                "value {v} outside depth range {depth}"
            );
        }
    }

    #[test]
    fn zero_depth_returns_zero() {
        let mut lfo = Lfo::new(LfoShape::Sine, 10.0, 0.0, 48000.0);
        for _ in 0..480 {
            assert_eq!(lfo.tick(), 0.0);
        }
    }

    #[test]
    fn set_shape_works() {
        let mut lfo = Lfo::default_sine(48000.0);
        assert_eq!(lfo.shape(), LfoShape::Sine);
        lfo.set_shape(LfoShape::Square);
        assert_eq!(lfo.shape(), LfoShape::Square);
    }
}
