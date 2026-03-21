//! ADSR envelope generator — backed by dhvani.

pub use dhvani::dsp::{AdsrParams, EnvelopeState};

/// ADSR envelope generator.
///
/// Call `trigger()` on note-on, `release()` on note-off, and `tick()` every sample.
#[derive(Debug, Clone)]
pub struct Envelope {
    sample_rate: f32,
    inner: dhvani::dsp::Envelope,
}

impl Envelope {
    pub fn new(params: AdsrParams, sample_rate: f32) -> Self {
        Self {
            sample_rate,
            inner: dhvani::dsp::Envelope::new(params, sample_rate as u32),
        }
    }

    /// Create with default ADSR parameters.
    pub fn default_adsr(sample_rate: f32) -> Self {
        Self::new(AdsrParams::default(), sample_rate)
    }

    /// Trigger the envelope (note-on).
    pub fn trigger(&mut self) {
        self.inner.trigger();
    }

    /// Release the envelope (note-off).
    pub fn release(&mut self) {
        self.inner.release();
    }

    /// Advance one sample and return the current level (0.0–1.0).
    pub fn tick(&mut self) -> f32 {
        self.inner.tick()
    }

    /// Current level without advancing.
    pub fn level(&self) -> f32 {
        self.inner.level()
    }

    /// Current envelope state.
    pub fn state(&self) -> EnvelopeState {
        self.inner.state()
    }

    /// Whether the envelope has finished (idle after release).
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn set_params(&mut self, params: AdsrParams) {
        self.inner.set_params(params);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.inner = dhvani::dsp::Envelope::new(AdsrParams::default(), sample_rate as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_returns_zero() {
        let mut env = Envelope::default_adsr(48000.0);
        assert_eq!(env.tick(), 0.0);
        assert_eq!(env.state(), EnvelopeState::Idle);
    }

    #[test]
    fn attack_ramps_up() {
        let mut env = Envelope::new(
            AdsrParams {
                attack: 0.01,
                decay: 0.1,
                sustain: 0.7,
                release: 0.3,
            },
            48000.0,
        );
        env.trigger();
        // After some samples, level should increase
        let mut prev = 0.0;
        let mut ramped = false;
        for _ in 0..480 {
            let v = env.tick();
            if v > prev + 0.001 {
                ramped = true;
            }
            prev = v;
        }
        assert!(ramped, "envelope should ramp up during attack");
    }

    #[test]
    fn release_decays() {
        let mut env = Envelope::new(
            AdsrParams {
                attack: 0.001,
                decay: 0.001,
                sustain: 0.7,
                release: 0.01,
            },
            48000.0,
        );
        env.trigger();
        // Run through attack+decay to reach sustain
        for _ in 0..4800 {
            env.tick();
        }
        let level_before = env.level();
        env.release();
        // Run through release
        for _ in 0..4800 {
            env.tick();
        }
        assert!(
            env.level() < level_before,
            "level should decay after release"
        );
    }

    #[test]
    fn full_adsr_cycle() {
        let mut env = Envelope::new(
            AdsrParams {
                attack: 0.001,
                decay: 0.001,
                sustain: 0.5,
                release: 0.001,
            },
            48000.0,
        );
        assert!(env.is_finished());
        env.trigger();
        assert!(!env.is_finished());
        // Run long enough to complete
        for _ in 0..48000 {
            env.tick();
        }
        env.release();
        for _ in 0..48000 {
            env.tick();
        }
        assert!(env.is_finished());
    }
}
