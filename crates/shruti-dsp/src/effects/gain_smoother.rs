//! Gain smoother — exponential moving average, backed by dhvani.

use crate::constants::{DEFAULT_GAIN_SMOOTHER_ATTACK, DEFAULT_GAIN_SMOOTHER_RELEASE};

pub use dhvani::dsp::GainSmootherParams;

/// Exponential moving average gain smoother.
///
/// Prevents pumping artifacts when applying per-buffer normalization.
#[derive(Debug, Clone)]
pub struct GainSmoother {
    inner: dhvani::dsp::GainSmoother,
}

impl GainSmoother {
    pub fn new(attack: f32, release: f32) -> Self {
        Self {
            inner: dhvani::dsp::GainSmoother::new(attack, release),
        }
    }

    /// Create with default parameters.
    pub fn default_params() -> Self {
        Self::new(DEFAULT_GAIN_SMOOTHER_ATTACK, DEFAULT_GAIN_SMOOTHER_RELEASE)
    }

    /// Smooth toward the target gain value. Call once per buffer.
    pub fn smooth(&mut self, target: f32) -> f32 {
        self.inner.smooth(target)
    }

    /// Current smoothed value.
    pub fn current(&self) -> f32 {
        self.inner.current()
    }

    /// Reset to a specific value (jump, no smoothing).
    pub fn reset(&mut self, value: f32) {
        self.inner.reset(value);
    }

    pub fn set_params(&mut self, params: GainSmootherParams) {
        self.inner.set_params(params);
    }

    pub fn params(&self) -> &GainSmootherParams {
        self.inner.params()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_value_is_one() {
        let gs = GainSmoother::default_params();
        assert_eq!(gs.current(), 1.0);
    }

    #[test]
    fn smooth_converges() {
        let mut gs = GainSmoother::new(0.5, 0.5);
        let target = 0.5;
        let mut prev = gs.current();
        for _ in 0..100 {
            let v = gs.smooth(target);
            // Should converge toward target
            assert!((v - target).abs() <= (prev - target).abs() + 0.001);
            prev = v;
        }
        assert!((gs.current() - target).abs() < 0.01);
    }

    #[test]
    fn reset_jumps() {
        let mut gs = GainSmoother::default_params();
        gs.reset(0.3);
        assert_eq!(gs.current(), 0.3);
    }

    #[test]
    fn params_roundtrip() {
        let gs = GainSmoother::new(0.4, 0.1);
        let p = gs.params();
        assert_eq!(p.attack, 0.4);
        assert_eq!(p.release, 0.1);
    }
}
