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
                    self.stage_pos = 0;
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
                    self.stage_pos = 0;
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

    /// Helper: count samples until envelope leaves a given state.
    fn samples_in_state(env: &mut Envelope, target_state: EnvelopeState, max: usize) -> usize {
        let mut count = 0;
        while env.state == target_state && count < max {
            env.tick();
            count += 1;
        }
        count
    }

    /// Helper: convert samples to milliseconds.
    fn samples_to_ms(samples: usize, sample_rate: f32) -> f32 {
        samples as f32 / sample_rate * 1000.0
    }

    #[test]
    fn attack_timing_44100() {
        let attack_s = 0.050; // 50ms
        let sr = 44100.0;
        let params = AdsrParams {
            attack: attack_s,
            decay: 0.001,
            sustain: 0.5,
            release: 0.01,
        };
        let mut env = Envelope::new(params, sr);
        env.trigger();
        let samples = samples_in_state(&mut env, EnvelopeState::Attack, 100000);
        let ms = samples_to_ms(samples, sr);
        let expected_ms = attack_s * 1000.0;
        assert!(
            (ms - expected_ms).abs() < 1.0,
            "attack at 44100: expected {expected_ms}ms, got {ms}ms"
        );
    }

    #[test]
    fn attack_timing_48000() {
        let attack_s = 0.050;
        let sr = 48000.0;
        let params = AdsrParams {
            attack: attack_s,
            decay: 0.001,
            sustain: 0.5,
            release: 0.01,
        };
        let mut env = Envelope::new(params, sr);
        env.trigger();
        let samples = samples_in_state(&mut env, EnvelopeState::Attack, 100000);
        let ms = samples_to_ms(samples, sr);
        let expected_ms = attack_s * 1000.0;
        assert!(
            (ms - expected_ms).abs() < 1.0,
            "attack at 48000: expected {expected_ms}ms, got {ms}ms"
        );
    }

    #[test]
    fn attack_timing_96000() {
        let attack_s = 0.050;
        let sr = 96000.0;
        let params = AdsrParams {
            attack: attack_s,
            decay: 0.001,
            sustain: 0.5,
            release: 0.01,
        };
        let mut env = Envelope::new(params, sr);
        env.trigger();
        let samples = samples_in_state(&mut env, EnvelopeState::Attack, 100000);
        let ms = samples_to_ms(samples, sr);
        let expected_ms = attack_s * 1000.0;
        assert!(
            (ms - expected_ms).abs() < 1.0,
            "attack at 96000: expected {expected_ms}ms, got {ms}ms"
        );
    }

    #[test]
    fn decay_timing_accuracy() {
        let decay_s = 0.100; // 100ms
        let sr = 48000.0;
        let params = AdsrParams {
            attack: 0.001,
            decay: decay_s,
            sustain: 0.5,
            release: 0.01,
        };
        let mut env = Envelope::new(params, sr);
        env.trigger();
        // Run through attack first
        samples_in_state(&mut env, EnvelopeState::Attack, 100000);
        // Now measure decay
        let samples = samples_in_state(&mut env, EnvelopeState::Decay, 100000);
        let ms = samples_to_ms(samples, sr);
        let expected_ms = decay_s * 1000.0;
        assert!(
            (ms - expected_ms).abs() < 1.0,
            "decay: expected {expected_ms}ms, got {ms}ms"
        );
    }

    #[test]
    fn release_timing_accuracy() {
        let release_s = 0.200; // 200ms
        let sr = 48000.0;
        let params = AdsrParams {
            attack: 0.001,
            decay: 0.001,
            sustain: 0.5,
            release: release_s,
        };
        let mut env = Envelope::new(params, sr);
        env.trigger();
        // Run through attack + decay to sustain
        for _ in 0..1000 {
            env.tick();
        }
        assert_eq!(env.state, EnvelopeState::Sustain);
        env.release();
        let samples = samples_in_state(&mut env, EnvelopeState::Release, 100000);
        let ms = samples_to_ms(samples, sr);
        let expected_ms = release_s * 1000.0;
        assert!(
            (ms - expected_ms).abs() < 1.0,
            "release: expected {expected_ms}ms, got {ms}ms"
        );
    }

    #[test]
    fn short_attack_timing() {
        // Very short attack: 1ms
        let attack_s = 0.001;
        let sr = 48000.0;
        let params = AdsrParams {
            attack: attack_s,
            decay: 0.01,
            sustain: 0.5,
            release: 0.01,
        };
        let mut env = Envelope::new(params, sr);
        env.trigger();
        let samples = samples_in_state(&mut env, EnvelopeState::Attack, 100000);
        let ms = samples_to_ms(samples, sr);
        let expected_ms = attack_s * 1000.0;
        assert!(
            (ms - expected_ms).abs() < 1.0,
            "1ms attack: expected {expected_ms}ms, got {ms}ms"
        );
    }

    #[test]
    fn sample_rate_change_adjusts_timing() {
        let attack_s = 0.050;
        let params = AdsrParams {
            attack: attack_s,
            decay: 0.01,
            sustain: 0.5,
            release: 0.01,
        };

        // At 44100
        let mut env1 = Envelope::new(params.clone(), 44100.0);
        env1.trigger();
        let samples1 = samples_in_state(&mut env1, EnvelopeState::Attack, 100000);

        // At 96000
        let mut env2 = Envelope::new(params, 96000.0);
        env2.trigger();
        let samples2 = samples_in_state(&mut env2, EnvelopeState::Attack, 100000);

        // Higher sample rate should need more samples for same time
        assert!(
            samples2 > samples1,
            "96kHz should need more samples than 44.1kHz: {samples2} vs {samples1}"
        );

        // But the time should be the same (within 1ms)
        let ms1 = samples_to_ms(samples1, 44100.0);
        let ms2 = samples_to_ms(samples2, 96000.0);
        assert!(
            (ms1 - ms2).abs() < 1.0,
            "timing should match across sample rates: {ms1}ms vs {ms2}ms"
        );
    }

    #[test]
    fn stage_pos_reset_on_decay_to_sustain() {
        // After transitioning from Decay to Sustain, stage_pos should be 0
        // so that a subsequent retrigger or release doesn't use stale position.
        let params = AdsrParams {
            attack: 0.001,
            decay: 0.001,
            sustain: 0.5,
            release: 0.01,
        };
        let mut env = Envelope::new(params, 48000.0);
        env.trigger();
        // Run through attack + decay to reach sustain
        for _ in 0..500 {
            env.tick();
        }
        assert_eq!(env.state, EnvelopeState::Sustain);
        assert_eq!(
            env.stage_pos, 0,
            "stage_pos should be 0 after Decay->Sustain transition"
        );
    }

    #[test]
    fn stage_pos_reset_on_release_to_idle() {
        // After transitioning from Release to Idle, stage_pos should be 0
        // so that retriggering starts cleanly.
        let params = AdsrParams {
            attack: 0.001,
            decay: 0.001,
            sustain: 0.5,
            release: 0.001,
        };
        let mut env = Envelope::new(params, 48000.0);
        env.trigger();
        for _ in 0..500 {
            env.tick();
        }
        env.release();
        for _ in 0..500 {
            env.tick();
        }
        assert_eq!(env.state, EnvelopeState::Idle);
        assert_eq!(
            env.stage_pos, 0,
            "stage_pos should be 0 after Release->Idle transition"
        );
    }

    #[test]
    fn retrigger_after_full_cycle_works_correctly() {
        // Ensure retrigger from Idle (after full ADSR cycle) produces
        // a correct attack ramp without stale stage_pos.
        let params = AdsrParams {
            attack: 0.01,
            decay: 0.01,
            sustain: 0.5,
            release: 0.01,
        };
        let mut env = Envelope::new(params, 1000.0);
        env.trigger();
        // Full cycle: attack -> decay -> sustain -> release -> idle
        for _ in 0..100 {
            env.tick();
        }
        env.release();
        for _ in 0..100 {
            env.tick();
        }
        assert_eq!(env.state, EnvelopeState::Idle);

        // Retrigger
        env.trigger();
        let first_level = env.tick();
        assert!(
            first_level < 0.2,
            "first sample after retrigger should be near start of attack, got {first_level}"
        );
    }
}
