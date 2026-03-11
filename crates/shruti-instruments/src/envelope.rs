use serde::{Deserialize, Serialize};

/// ADSR envelope parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdsrParams {
    /// Attack time in seconds.
    pub attack: f32,
    /// Decay time in seconds.
    pub decay: f32,
    /// Sustain level (0.0 to 1.0).
    pub sustain: f32,
    /// Release time in seconds.
    pub release: f32,
}

impl Default for AdsrParams {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
        }
    }
}

/// Current state of an ADSR envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// ADSR envelope generator.
pub struct Envelope {
    pub state: EnvelopeState,
    pub params: AdsrParams,
    pub level: f32,
    sample_rate: f32,
    /// Position within the current stage (in samples).
    stage_pos: u64,
    /// Level at the start of release (for smooth release from any level).
    release_start_level: f32,
}

impl Envelope {
    pub fn new(params: AdsrParams, sample_rate: f32) -> Self {
        Self {
            state: EnvelopeState::Idle,
            params,
            level: 0.0,
            sample_rate,
            stage_pos: 0,
            release_start_level: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Trigger the envelope (note on).
    pub fn trigger(&mut self) {
        self.state = EnvelopeState::Attack;
        self.stage_pos = 0;
    }

    /// Release the envelope (note off).
    pub fn release(&mut self) {
        if self.state != EnvelopeState::Idle {
            self.release_start_level = self.level;
            self.state = EnvelopeState::Release;
            self.stage_pos = 0;
        }
    }

    /// Process one sample. Returns the current envelope level.
    pub fn tick(&mut self) -> f32 {
        match self.state {
            EnvelopeState::Idle => {
                self.level = 0.0;
            }
            EnvelopeState::Attack => {
                let attack_samples = (self.params.attack * self.sample_rate).max(1.0) as u64;
                self.level = self.stage_pos as f32 / attack_samples as f32;
                self.stage_pos += 1;
                if self.stage_pos >= attack_samples {
                    self.level = 1.0;
                    self.state = EnvelopeState::Decay;
                    self.stage_pos = 0;
                }
            }
            EnvelopeState::Decay => {
                let decay_samples = (self.params.decay * self.sample_rate).max(1.0) as u64;
                let progress = self.stage_pos as f32 / decay_samples as f32;
                self.level = 1.0 + (self.params.sustain - 1.0) * progress;
                self.stage_pos += 1;
                if self.stage_pos >= decay_samples {
                    self.level = self.params.sustain;
                    self.state = EnvelopeState::Sustain;
                }
            }
            EnvelopeState::Sustain => {
                self.level = self.params.sustain;
            }
            EnvelopeState::Release => {
                let release_samples = (self.params.release * self.sample_rate).max(1.0) as u64;
                let progress = self.stage_pos as f32 / release_samples as f32;
                self.level = self.release_start_level * (1.0 - progress);
                self.stage_pos += 1;
                if self.stage_pos >= release_samples {
                    self.level = 0.0;
                    self.state = EnvelopeState::Idle;
                }
            }
        }
        self.level
    }

    /// Whether the envelope has finished (returned to idle after release).
    pub fn is_finished(&self) -> bool {
        self.state == EnvelopeState::Idle
    }

    /// Reset to idle state.
    pub fn reset(&mut self) {
        self.state = EnvelopeState::Idle;
        self.level = 0.0;
        self.stage_pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_params() {
        let p = AdsrParams::default();
        assert_eq!(p.attack, 0.01);
        assert_eq!(p.sustain, 0.7);
    }

    #[test]
    fn envelope_starts_idle() {
        let env = Envelope::new(AdsrParams::default(), 48000.0);
        assert_eq!(env.state, EnvelopeState::Idle);
        assert_eq!(env.level, 0.0);
    }

    #[test]
    fn envelope_attack_ramps_up() {
        let params = AdsrParams {
            attack: 0.01,
            decay: 0.01,
            sustain: 0.5,
            release: 0.01,
        };
        let mut env = Envelope::new(params, 1000.0); // 1kHz for easy math
        env.trigger();
        // 0.01s * 1000 = 10 samples for attack
        let mut levels = Vec::new();
        for _ in 0..10 {
            levels.push(env.tick());
        }
        // Should ramp from 0 to 1
        assert!(levels[0] < 0.2);
        assert!(levels.last().unwrap() >= &0.9);
    }

    #[test]
    fn envelope_reaches_sustain() {
        let params = AdsrParams {
            attack: 0.001,
            decay: 0.001,
            sustain: 0.6,
            release: 0.01,
        };
        let mut env = Envelope::new(params, 48000.0);
        env.trigger();
        // Run through attack + decay
        for _ in 0..500 {
            env.tick();
        }
        assert_eq!(env.state, EnvelopeState::Sustain);
        assert!((env.level - 0.6).abs() < 0.01);
    }

    #[test]
    fn envelope_release_to_idle() {
        let params = AdsrParams {
            attack: 0.001,
            decay: 0.001,
            sustain: 0.5,
            release: 0.01,
        };
        let mut env = Envelope::new(params, 48000.0);
        env.trigger();
        for _ in 0..500 {
            env.tick();
        }
        env.release();
        assert_eq!(env.state, EnvelopeState::Release);
        for _ in 0..1000 {
            env.tick();
        }
        assert_eq!(env.state, EnvelopeState::Idle);
        assert_eq!(env.level, 0.0);
    }

    #[test]
    fn envelope_is_finished() {
        let params = AdsrParams {
            attack: 0.001,
            decay: 0.001,
            sustain: 0.5,
            release: 0.001,
        };
        let mut env = Envelope::new(params, 48000.0);
        assert!(env.is_finished()); // idle = finished
        env.trigger();
        assert!(!env.is_finished());
        for _ in 0..500 {
            env.tick();
        }
        env.release();
        for _ in 0..500 {
            env.tick();
        }
        assert!(env.is_finished());
    }

    #[test]
    fn envelope_reset() {
        let mut env = Envelope::new(AdsrParams::default(), 48000.0);
        env.trigger();
        for _ in 0..100 {
            env.tick();
        }
        env.reset();
        assert_eq!(env.state, EnvelopeState::Idle);
        assert_eq!(env.level, 0.0);
    }
}
