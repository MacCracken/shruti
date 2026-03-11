use serde::{Deserialize, Serialize};

/// LFO waveform shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoShape {
    Sine,
    Triangle,
    Square,
    SawUp,
    SawDown,
    SampleAndHold,
}

/// Low-frequency oscillator for modulation.
///
/// Outputs values in the range `-depth..+depth`.
pub struct Lfo {
    pub shape: LfoShape,
    pub rate: f32,
    pub depth: f32,
    phase: f64,
    sample_rate: f32,
    sh_value: f32,
    rng_state: u32,
}

impl Lfo {
    pub fn new(shape: LfoShape, rate: f32, depth: f32, sample_rate: f32) -> Self {
        Self {
            shape,
            rate,
            depth: depth.clamp(0.0, 1.0),
            phase: 0.0,
            sample_rate,
            sh_value: 0.0,
            rng_state: 0x1234_5678,
        }
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr;
    }

    /// Advance the LFO by one sample and return the current value in `-depth..+depth`.
    pub fn tick(&mut self) -> f32 {
        let raw = match self.shape {
            LfoShape::Sine => (self.phase * std::f64::consts::TAU).sin() as f32,
            LfoShape::Triangle => {
                let p = self.phase as f32;
                if p < 0.25 {
                    p * 4.0
                } else if p < 0.75 {
                    2.0 - p * 4.0
                } else {
                    p * 4.0 - 4.0
                }
            }
            LfoShape::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            LfoShape::SawUp => (self.phase as f32) * 2.0 - 1.0,
            LfoShape::SawDown => 1.0 - (self.phase as f32) * 2.0,
            LfoShape::SampleAndHold => {
                // Update S&H value at each cycle start (phase wrap)
                let next_phase = self.phase + self.rate as f64 / self.sample_rate as f64;
                if next_phase >= 1.0 || self.phase == 0.0 {
                    // xorshift32
                    self.rng_state ^= self.rng_state << 13;
                    self.rng_state ^= self.rng_state >> 17;
                    self.rng_state ^= self.rng_state << 5;
                    self.sh_value = (self.rng_state as f32 / u32::MAX as f32) * 2.0 - 1.0;
                }
                self.sh_value
            }
        };

        // Advance phase
        self.phase += self.rate as f64 / self.sample_rate as f64;
        self.phase -= self.phase.floor();

        raw * self.depth
    }

    /// Reset LFO phase and S&H state.
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.sh_value = 0.0;
        self.rng_state = 0x1234_5678;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    #[test]
    fn sine_range() {
        let mut lfo = Lfo::new(LfoShape::Sine, 1.0, 1.0, SR);
        for _ in 0..(SR as usize) {
            let v = lfo.tick();
            assert!(
                (-1.01..=1.01).contains(&v),
                "Sine LFO out of range: {v}"
            );
        }
    }

    #[test]
    fn triangle_range() {
        let mut lfo = Lfo::new(LfoShape::Triangle, 1.0, 1.0, SR);
        for _ in 0..(SR as usize) {
            let v = lfo.tick();
            assert!(
                (-1.01..=1.01).contains(&v),
                "Triangle LFO out of range: {v}"
            );
        }
    }

    #[test]
    fn square_values() {
        let mut lfo = Lfo::new(LfoShape::Square, 1.0, 1.0, SR);
        for _ in 0..(SR as usize) {
            let v = lfo.tick();
            assert!(
                (v - 1.0).abs() < 0.01 || (v + 1.0).abs() < 0.01,
                "Square LFO should be +/-1, got {v}"
            );
        }
    }

    #[test]
    fn saw_up_range() {
        let mut lfo = Lfo::new(LfoShape::SawUp, 1.0, 1.0, SR);
        for _ in 0..(SR as usize) {
            let v = lfo.tick();
            assert!(
                (-1.01..=1.01).contains(&v),
                "SawUp LFO out of range: {v}"
            );
        }
    }

    #[test]
    fn saw_down_range() {
        let mut lfo = Lfo::new(LfoShape::SawDown, 1.0, 1.0, SR);
        for _ in 0..(SR as usize) {
            let v = lfo.tick();
            assert!(
                (-1.01..=1.01).contains(&v),
                "SawDown LFO out of range: {v}"
            );
        }
    }

    #[test]
    fn sample_and_hold_range() {
        let mut lfo = Lfo::new(LfoShape::SampleAndHold, 10.0, 1.0, SR);
        for _ in 0..(SR as usize) {
            let v = lfo.tick();
            assert!(
                (-1.01..=1.01).contains(&v),
                "S&H LFO out of range: {v}"
            );
        }
    }

    #[test]
    fn depth_scales_output() {
        let mut lfo = Lfo::new(LfoShape::Sine, 1.0, 0.5, SR);
        for _ in 0..(SR as usize) {
            let v = lfo.tick();
            assert!(
                (-0.51..=0.51).contains(&v),
                "Depth 0.5 should limit output to +/-0.5, got {v}"
            );
        }
    }

    #[test]
    fn rate_changes_frequency() {
        // With rate=1Hz, one full cycle takes SR samples.
        // Count zero-crossings for rate=1 vs rate=5; rate=5 should have ~5x as many.
        fn count_zero_crossings(rate: f32) -> usize {
            let mut lfo = Lfo::new(LfoShape::Sine, rate, 1.0, SR);
            let mut prev = lfo.tick();
            let mut crossings = 0;
            for _ in 1..(SR as usize) {
                let v = lfo.tick();
                if (prev >= 0.0 && v < 0.0) || (prev < 0.0 && v >= 0.0) {
                    crossings += 1;
                }
                prev = v;
            }
            crossings
        }
        let c1 = count_zero_crossings(1.0);
        let c5 = count_zero_crossings(5.0);
        // c5 should be roughly 5x c1 (with some tolerance)
        assert!(
            c5 > c1 * 3,
            "5Hz LFO should have more zero crossings than 1Hz: c1={c1}, c5={c5}"
        );
    }

    #[test]
    fn reset_restores_initial_state() {
        let mut lfo = Lfo::new(LfoShape::Sine, 1.0, 1.0, SR);
        // Advance some samples
        for _ in 0..1000 {
            lfo.tick();
        }
        lfo.reset();
        // After reset, first tick should be same as a fresh LFO
        let mut fresh = Lfo::new(LfoShape::Sine, 1.0, 1.0, SR);
        let v_reset = lfo.tick();
        let v_fresh = fresh.tick();
        assert!(
            (v_reset - v_fresh).abs() < 0.001,
            "Reset LFO should match fresh: reset={v_reset}, fresh={v_fresh}"
        );
    }
}
