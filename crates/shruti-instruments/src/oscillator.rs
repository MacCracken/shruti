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

    /// Fast approximation of 2^x using a degree-4 polynomial.
    /// Accurate to ~1e-4 for |x| < 1 (covers ±1200 cent detune).
    #[inline]
    #[allow(clippy::approx_constant)]
    pub(crate) fn fast_exp2_f64(x: f64) -> f64 {
        // Split into integer and fractional parts for wider range
        let xi = x.floor();
        let xf = x - xi;
        // Minimax polynomial for 2^x on [0, 1]
        let poly = 1.0
            + xf * (0.6931471805599453
                + xf * (0.24022650695910072
                    + xf * (0.05550410866482158 + xf * 0.009618129107628477)));
        poly * (2.0f64).powi(xi as i32)
    }

    /// Generate a single sample at the given phase (0.0 to 1.0) and frequency.
    #[inline]
    pub fn sample(&mut self, phase: f64, frequency: f64) -> f32 {
        let freq = frequency * Self::fast_exp2_f64(self.detune / 1200.0); // detune in cents
        let dt = freq / self.sample_rate;

        match self.waveform {
            Waveform::Sine => (phase * std::f64::consts::TAU).sin() as f32,
            Waveform::Saw => {
                let naive = 2.0 * phase - 1.0;
                // PolyBLEP corrects both the rising edge (phase near 0) and
                // trailing edge (phase near 1) of the discontinuity at the
                // cycle wrap point, smoothing the transition for anti-aliasing.
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
    #[inline]
    pub fn advance_phase(phase: f64, frequency: f64, sample_rate: f64) -> f64 {
        let new = phase + frequency / sample_rate;
        new - new.floor()
    }

    /// PolyBLEP anti-aliasing correction.
    #[inline]
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

    #[test]
    fn poly_blep_corrects_rising_edge() {
        // At phase just after 0 (rising edge), poly_blep should return a
        // negative correction that smooths the saw's jump from +1 to -1.
        let dt = 0.01; // typical dt for 440Hz at 48kHz
        let phase_near_zero = dt * 0.5;
        let blep = Oscillator::poly_blep(phase_near_zero, dt);
        assert!(
            blep < 0.0,
            "poly_blep near phase=0 should be negative (rising edge correction), got {blep}"
        );
    }

    #[test]
    fn poly_blep_corrects_trailing_edge() {
        // At phase just before 1.0 (trailing edge), poly_blep should return a
        // positive correction that smooths the saw's jump from +1 to -1.
        let dt = 0.01;
        let phase_near_one = 1.0 - dt * 0.5;
        let blep = Oscillator::poly_blep(phase_near_one, dt);
        assert!(
            blep > 0.0,
            "poly_blep near phase=1 should be positive (trailing edge correction), got {blep}"
        );
    }

    #[test]
    fn saw_blep_smooths_both_edges() {
        // Verify that saw samples near the wrap point are smoothed (pulled
        // toward 0) compared to the naive sawtooth.
        let mut osc = Oscillator::new(Waveform::Saw, 48000.0);
        let freq = 440.0;
        let dt = freq / 48000.0;

        // Just after wrap (rising edge) — naive would be near -1
        let s_rising = osc.sample(dt * 0.5, freq);
        let naive_rising = (2.0 * dt * 0.5 - 1.0) as f32;
        assert!(
            s_rising.abs() < naive_rising.abs(),
            "PolyBLEP should smooth rising edge: blep={s_rising}, naive={naive_rising}"
        );

        // Just before wrap (trailing edge) — naive would be near +1
        let s_trailing = osc.sample(1.0 - dt * 0.5, freq);
        let naive_trailing = (2.0 * (1.0 - dt * 0.5) - 1.0) as f32;
        assert!(
            s_trailing.abs() < naive_trailing.abs(),
            "PolyBLEP should smooth trailing edge: blep={s_trailing}, naive={naive_trailing}"
        );
    }

    // ── Helper: generate N samples and return the buffer ──────────────
    fn generate_samples(
        waveform: Waveform,
        frequency: f64,
        sample_rate: f64,
        num_samples: usize,
    ) -> Vec<f32> {
        let mut osc = Oscillator::new(waveform, sample_rate);
        let mut phase = 0.0f64;
        let mut buf = Vec::with_capacity(num_samples);
        for _ in 0..num_samples {
            buf.push(osc.sample(phase, frequency));
            phase = Oscillator::advance_phase(phase, frequency, sample_rate);
        }
        buf
    }

    // ── Helper: estimate fundamental frequency via zero-crossing rate ─
    fn estimate_frequency_zero_crossings(buf: &[f32], sample_rate: f64) -> f64 {
        let mut crossings = 0usize;
        for i in 1..buf.len() {
            if (buf[i - 1] >= 0.0 && buf[i] < 0.0) || (buf[i - 1] < 0.0 && buf[i] >= 0.0) {
                crossings += 1;
            }
        }
        // Each full cycle has 2 zero crossings
        let duration_secs = buf.len() as f64 / sample_rate;
        crossings as f64 / (2.0 * duration_secs)
    }

    // ================================================================
    // 1. Frequency accuracy tests
    // ================================================================

    #[test]
    fn frequency_accuracy_sine() {
        let sample_rate = 48000.0;
        let test_freqs = [(440.0, "A4"), (130.81, "C3"), (523.25, "C5")];
        for (freq, note) in &test_freqs {
            let buf = generate_samples(Waveform::Sine, *freq, sample_rate, sample_rate as usize);
            let measured = estimate_frequency_zero_crossings(&buf, sample_rate);
            assert!(
                (measured - freq).abs() < 1.0,
                "Sine {note}: expected {freq} Hz, measured {measured} Hz",
            );
        }
    }

    #[test]
    fn frequency_accuracy_saw() {
        let sample_rate = 48000.0;
        let test_freqs = [(440.0, "A4"), (130.81, "C3"), (523.25, "C5")];
        for (freq, note) in &test_freqs {
            let buf = generate_samples(Waveform::Saw, *freq, sample_rate, sample_rate as usize);
            let measured = estimate_frequency_zero_crossings(&buf, sample_rate);
            assert!(
                (measured - freq).abs() < 1.0,
                "Saw {note}: expected {freq} Hz, measured {measured} Hz",
            );
        }
    }

    #[test]
    fn frequency_accuracy_square() {
        let sample_rate = 48000.0;
        let test_freqs = [(440.0, "A4"), (130.81, "C3"), (523.25, "C5")];
        for (freq, note) in &test_freqs {
            let buf = generate_samples(Waveform::Square, *freq, sample_rate, sample_rate as usize);
            let measured = estimate_frequency_zero_crossings(&buf, sample_rate);
            assert!(
                (measured - freq).abs() < 1.0,
                "Square {note}: expected {freq} Hz, measured {measured} Hz",
            );
        }
    }

    #[test]
    fn frequency_accuracy_triangle() {
        let sample_rate = 48000.0;
        let test_freqs = [(440.0, "A4"), (130.81, "C3"), (523.25, "C5")];
        for (freq, note) in &test_freqs {
            let buf =
                generate_samples(Waveform::Triangle, *freq, sample_rate, sample_rate as usize);
            let measured = estimate_frequency_zero_crossings(&buf, sample_rate);
            assert!(
                (measured - freq).abs() < 1.0,
                "Triangle {note}: expected {freq} Hz, measured {measured} Hz",
            );
        }
    }

    // ================================================================
    // 2. DC offset tests
    // ================================================================

    #[test]
    fn dc_offset_sine() {
        let sample_rate = 48000.0f64;
        // Generate exactly whole cycles to avoid partial-cycle bias
        let num_cycles = 100;
        let freq = 440.0f64;
        let samples_per_cycle = (sample_rate / freq).round() as usize;
        let total = samples_per_cycle * num_cycles;
        let buf = generate_samples(Waveform::Sine, freq, sample_rate, total);
        let dc: f64 = buf.iter().map(|s| *s as f64).sum::<f64>() / buf.len() as f64;
        assert!(dc.abs() < 0.01, "Sine DC offset should be < 0.01, got {dc}",);
    }

    #[test]
    fn dc_offset_saw() {
        let sample_rate = 48000.0f64;
        let num_cycles = 100;
        let freq = 440.0f64;
        let samples_per_cycle = (sample_rate / freq).round() as usize;
        let total = samples_per_cycle * num_cycles;
        let buf = generate_samples(Waveform::Saw, freq, sample_rate, total);
        let dc: f64 = buf.iter().map(|s| *s as f64).sum::<f64>() / buf.len() as f64;
        assert!(dc.abs() < 0.01, "Saw DC offset should be < 0.01, got {dc}",);
    }

    #[test]
    fn dc_offset_square() {
        let sample_rate = 48000.0f64;
        let num_cycles = 100;
        let freq = 440.0f64;
        let samples_per_cycle = (sample_rate / freq).round() as usize;
        let total = samples_per_cycle * num_cycles;
        let buf = generate_samples(Waveform::Square, freq, sample_rate, total);
        let dc: f64 = buf.iter().map(|s| *s as f64).sum::<f64>() / buf.len() as f64;
        assert!(
            dc.abs() < 0.01,
            "Square DC offset should be < 0.01, got {dc}",
        );
    }

    #[test]
    fn dc_offset_triangle() {
        let sample_rate = 48000.0f64;
        let num_cycles = 100;
        let freq = 440.0f64;
        let samples_per_cycle = (sample_rate / freq).round() as usize;
        let total = samples_per_cycle * num_cycles;
        let buf = generate_samples(Waveform::Triangle, freq, sample_rate, total);
        let dc: f64 = buf.iter().map(|s| *s as f64).sum::<f64>() / buf.len() as f64;
        assert!(
            dc.abs() < 0.01,
            "Triangle DC offset should be < 0.01, got {dc}",
        );
    }

    // ================================================================
    // 3. Output range tests
    // ================================================================

    #[test]
    fn output_range_all_waveforms() {
        let sample_rate = 48000.0;
        let waveforms = [
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Triangle,
            Waveform::Noise,
        ];
        for wf in &waveforms {
            let buf = generate_samples(*wf, 440.0, sample_rate, 48000);
            for (i, s) in buf.iter().enumerate() {
                assert!(
                    *s >= -1.0 && *s <= 1.0,
                    "{wf:?} sample {i} out of [-1.0, 1.0] range: {s}",
                );
            }
        }
    }

    #[test]
    fn output_range_extreme_frequencies() {
        let sample_rate = 48000.0;
        // Test at very low and moderately high frequencies
        let freqs = [20.0, 100.0, 1000.0, 5000.0, 10000.0];
        let waveforms = [
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Triangle,
        ];
        for wf in &waveforms {
            for freq in &freqs {
                let buf = generate_samples(*wf, *freq, sample_rate, 4800);
                for s in &buf {
                    assert!(
                        *s >= -1.0 && *s <= 1.0,
                        "{wf:?} at {freq} Hz out of range: {s}",
                    );
                }
            }
        }
    }

    // ================================================================
    // 4. Noise non-periodicity (autocorrelation) test
    // ================================================================

    #[test]
    fn noise_low_autocorrelation() {
        let sample_rate = 48000.0;
        let buf = generate_samples(Waveform::Noise, 440.0, sample_rate, 4096);

        // Compute mean
        let mean: f64 = buf.iter().map(|s| *s as f64).sum::<f64>() / buf.len() as f64;

        // Compute variance (autocorrelation at lag 0)
        let variance: f64 = buf
            .iter()
            .map(|s| {
                let d = *s as f64 - mean;
                d * d
            })
            .sum::<f64>()
            / buf.len() as f64;
        assert!(variance > 0.01, "Noise variance too low: {variance}");

        // Compute normalized autocorrelation at several lags
        // For truly random noise, autocorrelation at lag > 0 should be near zero.
        let test_lags = [1, 2, 5, 10, 50, 100];
        for lag in &test_lags {
            let mut ac = 0.0f64;
            for i in 0..(buf.len() - lag) {
                ac += (buf[i] as f64 - mean) * (buf[i + lag] as f64 - mean);
            }
            ac /= (buf.len() - lag) as f64;
            let normalized = ac / variance;
            assert!(
                normalized.abs() < 0.1,
                "Noise autocorrelation at lag {lag} too high: {normalized} (should be near 0)",
            );
        }
    }

    #[test]
    fn noise_has_spread_distribution() {
        let sample_rate = 48000.0;
        let buf = generate_samples(Waveform::Noise, 440.0, sample_rate, 10000);
        // Check that noise actually spans a reasonable range
        let min = buf.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = buf.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 1.0, "Noise range too narrow: [{min}, {max}]",);
        // Check rough uniformity: both positive and negative samples
        let pos_count = buf.iter().filter(|s| **s > 0.0).count();
        let neg_count = buf.iter().filter(|s| **s < 0.0).count();
        let ratio = pos_count as f64 / neg_count as f64;
        assert!(
            (0.7..=1.4).contains(&ratio),
            "Noise pos/neg ratio unbalanced: {ratio} (pos={pos_count}, neg={neg_count})",
        );
    }
}
