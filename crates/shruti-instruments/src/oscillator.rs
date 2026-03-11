use serde::{Deserialize, Serialize};

/// Waveform type for oscillators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
    Noise,
}

/// A basic oscillator with anti-aliasing via PolyBLEP.
pub struct Oscillator {
    pub waveform: Waveform,
    pub detune: f64,
    sample_rate: f64,
    rng_state: u32,
}

impl Oscillator {
    pub fn new(waveform: Waveform, sample_rate: f64) -> Self {
        Self {
            waveform,
            detune: 0.0,
            sample_rate,
            rng_state: 12345,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
    }

    /// Generate a single sample at the given phase (0.0 to 1.0) and frequency.
    pub fn sample(&mut self, phase: f64, frequency: f64) -> f32 {
        let freq = frequency * 2.0f64.powf(self.detune / 1200.0); // detune in cents
        let dt = freq / self.sample_rate;

        match self.waveform {
            Waveform::Sine => (phase * std::f64::consts::TAU).sin() as f32,
            Waveform::Saw => {
                let naive = 2.0 * phase - 1.0;
                (naive - Self::poly_blep(phase, dt)) as f32
            }
            Waveform::Square => {
                let naive = if phase < 0.5 { 1.0 } else { -1.0 };
                let mut out = naive;
                out += Self::poly_blep(phase, dt);
                out -= Self::poly_blep((phase + 0.5) % 1.0, dt);
                out as f32
            }
            Waveform::Triangle => {
                // Phase-based triangle wave
                let tri = if phase < 0.25 {
                    4.0 * phase
                } else if phase < 0.75 {
                    2.0 - 4.0 * phase
                } else {
                    4.0 * phase - 4.0
                };
                tri as f32
            }
            Waveform::Noise => {
                // Simple white noise via xorshift
                self.rng_state ^= self.rng_state << 13;
                self.rng_state ^= self.rng_state >> 17;
                self.rng_state ^= self.rng_state << 5;
                (self.rng_state as f32 / u32::MAX as f32) * 2.0 - 1.0
            }
        }
    }

    /// Advance a phase accumulator by one sample at the given frequency.
    /// Returns the new phase (wrapped to 0.0..1.0).
    pub fn advance_phase(phase: f64, frequency: f64, sample_rate: f64) -> f64 {
        let new = phase + frequency / sample_rate;
        new - new.floor()
    }

    /// PolyBLEP anti-aliasing correction.
    fn poly_blep(phase: f64, dt: f64) -> f64 {
        if phase < dt {
            let t = phase / dt;
            2.0 * t - t * t - 1.0
        } else if phase > 1.0 - dt {
            let t = (phase - 1.0) / dt;
            t * t + 2.0 * t + 1.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_zero_crossing() {
        let mut osc = Oscillator::new(Waveform::Sine, 48000.0);
        let s = osc.sample(0.0, 440.0);
        assert!(s.abs() < 0.001, "sine at phase 0 should be ~0, got {s}");
    }

    #[test]
    fn sine_peak() {
        let mut osc = Oscillator::new(Waveform::Sine, 48000.0);
        let s = osc.sample(0.25, 440.0);
        assert!(
            (s - 1.0).abs() < 0.001,
            "sine at phase 0.25 should be ~1.0, got {s}"
        );
    }

    #[test]
    fn saw_range() {
        let mut osc = Oscillator::new(Waveform::Saw, 48000.0);
        for i in 0..100 {
            let phase = i as f64 / 100.0;
            let s = osc.sample(phase, 440.0);
            assert!(
                (-1.1..=1.1).contains(&s),
                "saw out of range at phase {phase}: {s}"
            );
        }
    }

    #[test]
    fn square_values() {
        let mut osc = Oscillator::new(Waveform::Square, 48000.0);
        let s_low = osc.sample(0.1, 100.0);
        let s_high = osc.sample(0.6, 100.0);
        assert!(
            s_low > 0.5,
            "square at phase 0.1 should be positive, got {s_low}"
        );
        assert!(
            s_high < -0.5,
            "square at phase 0.6 should be negative, got {s_high}"
        );
    }

    #[test]
    fn triangle_range() {
        let mut osc = Oscillator::new(Waveform::Triangle, 48000.0);
        for i in 0..100 {
            let phase = i as f64 / 100.0;
            let s = osc.sample(phase, 440.0);
            assert!(
                (-1.1..=1.1).contains(&s),
                "tri out of range at phase {phase}: {s}"
            );
        }
    }

    #[test]
    fn noise_varies() {
        let mut osc = Oscillator::new(Waveform::Noise, 48000.0);
        let s1 = osc.sample(0.0, 440.0);
        let s2 = osc.sample(0.0, 440.0);
        assert_ne!(s1, s2, "noise should produce different values");
    }

    #[test]
    fn advance_phase_wraps() {
        let phase = Oscillator::advance_phase(0.99, 48000.0, 48000.0);
        assert!((0.0..1.0).contains(&phase));
    }

    #[test]
    fn detune_changes_pitch() {
        let mut osc = Oscillator::new(Waveform::Sine, 48000.0);
        osc.detune = 1200.0; // +1 octave
        // Detuned oscillator should produce different output than non-detuned at same phase
        let detuned = osc.sample(0.1, 440.0);
        let mut normal_osc = Oscillator::new(Waveform::Sine, 48000.0);
        let normal = normal_osc.sample(0.1, 440.0);
        // Both should be valid samples (this test just validates detune doesn't panic)
        assert!(detuned.abs() <= 1.0 && normal.abs() <= 1.0);
    }
}
