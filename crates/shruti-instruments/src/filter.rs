use serde::{Deserialize, Serialize};

/// Filter mode for the state-variable filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterMode {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

/// A state-variable filter (SVF) based on the Cytomic/Andrew Simper topology.
///
/// Supports low-pass, high-pass, band-pass, and notch modes with
/// adjustable cutoff frequency and resonance.
///
/// # Cutoff modulation
///
/// When used inside [`SubtractiveSynth`](crate::synth::SubtractiveSynth),
/// the cutoff frequency is modulated in **octaves** by both the filter
/// envelope and LFO:
///
/// ```text
/// modulated_cutoff = base_cutoff * 2^(env_mod + lfo_mod)
/// ```
///
/// - **Filter envelope depth** (`FilterEnvDepth`, -1.0 to +1.0) scales
///   the envelope output by up to **4 octaves** in either direction.
///   A depth of +1.0 means the envelope sweeps the cutoff up by 4
///   octaves (16x frequency) at its peak; -1.0 sweeps down by 4 octaves.
///
/// - **LFO cutoff modulation** similarly contributes up to **4 octaves**
///   of bipolar modulation based on the LFO's current output and depth.
///
/// The final modulated cutoff is clamped to the 20 Hz – 20 kHz range.
pub struct Filter {
    pub mode: FilterMode,
    pub cutoff: f32,
    pub resonance: f32,
    sample_rate: f32,
    // SVF state variables
    ic1eq: f32,
    ic2eq: f32,
    // Cached coefficients (recomputed when cutoff/resonance change)
    cached_cutoff: f32,
    cached_resonance: f32,
    cached_g: f32,
    cached_k: f32,
    cached_a1: f32,
    cached_a2: f32,
    cached_a3: f32,
}

impl Filter {
    pub fn new(mode: FilterMode, cutoff: f32, resonance: f32, sample_rate: f32) -> Self {
        let cutoff = cutoff.clamp(20.0, 20000.0);
        let resonance = resonance.clamp(0.0, 1.0);
        let g = (std::f32::consts::PI * cutoff / sample_rate).tan();
        let k = 2.0 - 2.0 * resonance;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        Self {
            mode,
            cutoff,
            resonance,
            sample_rate,
            ic1eq: 0.0,
            ic2eq: 0.0,
            cached_cutoff: cutoff,
            cached_resonance: resonance,
            cached_g: g,
            cached_k: k,
            cached_a1: a1,
            cached_a2: a2,
            cached_a3: a3,
        }
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr;
        self.update_coefficients();
    }

    /// Recompute coefficients when cutoff or resonance have changed.
    #[inline]
    fn update_coefficients(&mut self) {
        let clamped_res = self.resonance.clamp(0.0, 1.0);
        self.cached_g = (std::f32::consts::PI * self.cutoff.clamp(20.0, self.sample_rate * 0.49)
            / self.sample_rate)
            .tan();
        self.cached_k = 2.0 - 2.0 * clamped_res;
        self.cached_a1 = 1.0 / (1.0 + self.cached_g * (self.cached_g + self.cached_k));
        self.cached_a2 = self.cached_g * self.cached_a1;
        self.cached_a3 = self.cached_g * self.cached_a2;
        self.cached_cutoff = self.cutoff;
        self.cached_resonance = self.resonance;
    }

    /// Process a single sample through the SVF.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        // Only recompute coefficients if cutoff or resonance changed
        if self.cutoff != self.cached_cutoff || self.resonance != self.cached_resonance {
            self.update_coefficients();
        }

        let v3 = input - self.ic2eq;
        let v1 = self.cached_a1 * self.ic1eq + self.cached_a2 * v3;
        let v2 = self.ic2eq + self.cached_a2 * self.ic1eq + self.cached_a3 * v3;

        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        match self.mode {
            FilterMode::LowPass => v2,
            FilterMode::HighPass => input - self.cached_k * v1 - v2,
            FilterMode::BandPass => v1,
            FilterMode::Notch => input - self.cached_k * v1,
        }
    }

    /// Reset internal state variables to zero.
    pub fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    /// Generate a sine wave and measure RMS after filtering.
    fn measure_filtered_rms(mode: FilterMode, cutoff: f32, resonance: f32, freq: f32) -> f32 {
        let mut filter = Filter::new(mode, cutoff, resonance, SR);
        let num_samples = SR as usize; // 1 second
        let mut sum_sq = 0.0f64;
        // Skip initial transient
        let skip = (SR * 0.1) as usize;
        for i in 0..num_samples {
            let phase = 2.0 * std::f32::consts::PI * freq * i as f32 / SR;
            let input = phase.sin();
            let output = filter.process_sample(input);
            if i >= skip {
                sum_sq += (output as f64) * (output as f64);
            }
        }
        ((sum_sq / (num_samples - skip) as f64).sqrt()) as f32
    }

    #[test]
    fn lowpass_passes_low_frequencies() {
        let rms = measure_filtered_rms(FilterMode::LowPass, 5000.0, 0.0, 100.0);
        // 100 Hz through a 5000 Hz lowpass should pass mostly unattenuated
        assert!(rms > 0.5, "LP should pass 100Hz, got rms={rms}");
    }

    #[test]
    fn lowpass_attenuates_high_frequencies() {
        let rms = measure_filtered_rms(FilterMode::LowPass, 200.0, 0.0, 10000.0);
        // 10000 Hz through a 200 Hz lowpass should be heavily attenuated
        assert!(rms < 0.1, "LP should attenuate 10kHz, got rms={rms}");
    }

    #[test]
    fn highpass_passes_high_frequencies() {
        let rms = measure_filtered_rms(FilterMode::HighPass, 200.0, 0.0, 5000.0);
        assert!(rms > 0.4, "HP should pass 5kHz, got rms={rms}");
    }

    #[test]
    fn highpass_attenuates_low_frequencies() {
        let rms = measure_filtered_rms(FilterMode::HighPass, 5000.0, 0.0, 100.0);
        assert!(rms < 0.1, "HP should attenuate 100Hz, got rms={rms}");
    }

    #[test]
    fn bandpass_passes_center_frequency() {
        let rms = measure_filtered_rms(FilterMode::BandPass, 1000.0, 0.5, 1000.0);
        assert!(rms > 0.2, "BP should pass center freq, got rms={rms}");
    }

    #[test]
    fn bandpass_attenuates_far_frequency() {
        let rms_low = measure_filtered_rms(FilterMode::BandPass, 1000.0, 0.5, 50.0);
        let rms_high = measure_filtered_rms(FilterMode::BandPass, 1000.0, 0.5, 15000.0);
        assert!(
            rms_low < 0.15,
            "BP should attenuate 50Hz with cutoff 1kHz, got rms={rms_low}"
        );
        assert!(
            rms_high < 0.15,
            "BP should attenuate 15kHz with cutoff 1kHz, got rms={rms_high}"
        );
    }

    #[test]
    fn notch_attenuates_center_frequency() {
        let rms_center = measure_filtered_rms(FilterMode::Notch, 1000.0, 0.95, 1000.0);
        let rms_away = measure_filtered_rms(FilterMode::Notch, 1000.0, 0.95, 100.0);
        assert!(
            rms_center < rms_away,
            "Notch should attenuate center more than away: center={rms_center}, away={rms_away}"
        );
    }

    #[test]
    fn resonance_boosts_near_cutoff() {
        let rms_no_res = measure_filtered_rms(FilterMode::LowPass, 1000.0, 0.0, 900.0);
        let rms_high_res = measure_filtered_rms(FilterMode::LowPass, 1000.0, 0.95, 900.0);
        assert!(
            rms_high_res > rms_no_res,
            "High resonance should boost near cutoff: no_res={rms_no_res}, high_res={rms_high_res}"
        );
    }

    #[test]
    fn reset_clears_state() {
        let mut filter = Filter::new(FilterMode::LowPass, 1000.0, 0.5, SR);
        // Feed some signal
        for i in 0..1000 {
            let phase = 2.0 * std::f32::consts::PI * 440.0 * i as f32 / SR;
            filter.process_sample(phase.sin());
        }
        filter.reset();
        // After reset, the filter state should be zero, so output of 0 input = 0
        let out = filter.process_sample(0.0);
        assert!(
            out.abs() < f32::EPSILON,
            "After reset, 0 input should give 0 output, got {out}"
        );
    }

    #[test]
    fn set_sample_rate_updates_coefficients() {
        let mut filter = Filter::new(FilterMode::LowPass, 1000.0, 0.5, 48000.0);
        let g_before = filter.cached_g;
        filter.set_sample_rate(96000.0);
        // Doubling sample rate with same cutoff should halve g (approximately)
        assert!(
            filter.cached_g < g_before,
            "g should decrease at higher sample rate"
        );
    }

    #[test]
    fn dynamic_cutoff_change_triggers_recompute() {
        let mut filter = Filter::new(FilterMode::LowPass, 1000.0, 0.0, SR);
        let g_before = filter.cached_g;
        filter.cutoff = 5000.0;
        // Processing should trigger recompute
        filter.process_sample(1.0);
        assert_ne!(filter.cached_g, g_before);
        assert_eq!(filter.cached_cutoff, 5000.0);
    }

    #[test]
    fn resonance_above_one_clamped() {
        let mut filter = Filter::new(FilterMode::LowPass, 1000.0, 0.5, SR);
        filter.resonance = 1.5; // out of range
        filter.process_sample(1.0); // triggers recompute with clamping
        // k = 2 - 2*clamp(1.5, 0, 1) = 2 - 2 = 0
        assert!(
            filter.cached_k.abs() < f32::EPSILON,
            "k should be 0 for resonance=1.0 (clamped from 1.5)"
        );
    }

    #[test]
    fn cutoff_clamped_to_nyquist() {
        let mut filter = Filter::new(FilterMode::LowPass, 1000.0, 0.0, SR);
        filter.cutoff = 100_000.0; // way above nyquist
        filter.process_sample(1.0);
        // Should not produce NaN/infinity
        assert!(
            filter.cached_g.is_finite(),
            "g should be finite even with extreme cutoff"
        );
    }

    #[test]
    fn output_is_finite_under_stress() {
        let mut filter = Filter::new(FilterMode::LowPass, 1000.0, 0.99, SR);
        for i in 0..10000 {
            let input = if i % 2 == 0 { 1.0 } else { -1.0 };
            let out = filter.process_sample(input);
            assert!(out.is_finite(), "Output should be finite at sample {i}");
        }
    }

    /// Generate a sine wave, filter it, and return the RMS of the steady-state output.
    /// Uses 2 seconds of audio and skips the first 0.2s to avoid transient artifacts.
    fn sine_rms_through_filter(
        mode: FilterMode,
        cutoff: f32,
        resonance: f32,
        freq: f32,
        sample_rate: f32,
    ) -> f32 {
        let mut filter = Filter::new(mode, cutoff, resonance, sample_rate);
        let n = (sample_rate * 2.0) as usize;
        let skip = (sample_rate * 0.2) as usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin())
            .collect();
        let mut sum_sq = 0.0f64;
        let mut count = 0usize;
        for (i, &s) in samples.iter().enumerate() {
            let out = filter.process_sample(s);
            if i >= skip {
                sum_sq += (out as f64) * (out as f64);
                count += 1;
            }
        }
        (sum_sq / count as f64).sqrt() as f32
    }

    /// RMS of an unfiltered unit-amplitude sine is 1/sqrt(2) ≈ 0.7071
    const SINE_RMS: f32 = std::f32::consts::FRAC_1_SQRT_2;

    /// Convert a linear ratio to decibels.
    fn to_db(ratio: f32) -> f32 {
        20.0 * ratio.log10()
    }

    // ── Lowpass attenuation ──────────────────────────────────────────

    #[test]
    fn lowpass_attenuation_passes_below_cutoff() {
        let rms_500 = sine_rms_through_filter(FilterMode::LowPass, 1000.0, 0.0, 500.0, SR);
        // 500 Hz through 1 kHz lowpass should pass with minimal attenuation
        assert!(
            rms_500 > 0.5,
            "LP 1kHz cutoff should pass 500Hz nearly unattenuated, got rms={rms_500}"
        );
    }

    #[test]
    fn lowpass_attenuation_rejects_above_cutoff() {
        let rms_500 = sine_rms_through_filter(FilterMode::LowPass, 1000.0, 0.0, 500.0, SR);
        let rms_5k = sine_rms_through_filter(FilterMode::LowPass, 1000.0, 0.0, 5000.0, SR);
        let atten_db = to_db(rms_5k / rms_500);
        assert!(
            atten_db < -6.0,
            "LP 1kHz cutoff: 5kHz should be at least -6dB relative to 500Hz, got {atten_db:.1} dB"
        );
    }

    // ── Highpass attenuation ─────────────────────────────────────────

    #[test]
    fn highpass_attenuation_passes_above_cutoff() {
        let rms_5k = sine_rms_through_filter(FilterMode::HighPass, 1000.0, 0.0, 5000.0, SR);
        assert!(
            rms_5k > 0.5,
            "HP 1kHz cutoff should pass 5kHz nearly unattenuated, got rms={rms_5k}"
        );
    }

    #[test]
    fn highpass_attenuation_rejects_below_cutoff() {
        let rms_500 = sine_rms_through_filter(FilterMode::HighPass, 1000.0, 0.0, 500.0, SR);
        let rms_5k = sine_rms_through_filter(FilterMode::HighPass, 1000.0, 0.0, 5000.0, SR);
        let atten_db = to_db(rms_500 / rms_5k);
        assert!(
            atten_db < -6.0,
            "HP 1kHz cutoff: 500Hz should be at least -6dB relative to 5kHz, got {atten_db:.1} dB"
        );
    }

    // ── Bandpass peak ────────────────────────────────────────────────

    #[test]
    fn bandpass_peak_passes_center() {
        let rms_center = sine_rms_through_filter(FilterMode::BandPass, 1000.0, 0.5, 1000.0, SR);
        let rms_low = sine_rms_through_filter(FilterMode::BandPass, 1000.0, 0.5, 100.0, SR);
        let rms_high = sine_rms_through_filter(FilterMode::BandPass, 1000.0, 0.5, 10000.0, SR);
        assert!(
            rms_center > rms_low,
            "BP: center 1kHz rms={rms_center} should exceed 100Hz rms={rms_low}"
        );
        assert!(
            rms_center > rms_high,
            "BP: center 1kHz rms={rms_center} should exceed 10kHz rms={rms_high}"
        );
        // Both off-center should be attenuated meaningfully
        assert!(
            to_db(rms_low / rms_center) < -6.0,
            "BP: 100Hz should be at least -6dB vs center"
        );
        assert!(
            to_db(rms_high / rms_center) < -6.0,
            "BP: 10kHz should be at least -6dB vs center"
        );
    }

    // ── Notch rejection ──────────────────────────────────────────────

    #[test]
    fn notch_rejection_attenuates_center() {
        // High resonance for a tight notch
        let rms_center = sine_rms_through_filter(FilterMode::Notch, 1000.0, 0.99, 1000.0, SR);
        let rms_100 = sine_rms_through_filter(FilterMode::Notch, 1000.0, 0.99, 100.0, SR);
        let rms_5k = sine_rms_through_filter(FilterMode::Notch, 1000.0, 0.99, 5000.0, SR);
        // Center frequency should be strongly attenuated
        assert!(
            rms_center < 0.3,
            "Notch should attenuate 1kHz center, got rms={rms_center}"
        );
        // Away from center should pass relatively unchanged
        assert!(rms_100 > 0.5, "Notch should pass 100Hz, got rms={rms_100}");
        assert!(rms_5k > 0.5, "Notch should pass 5kHz, got rms={rms_5k}");
    }

    // ── Resonance boost ──────────────────────────────────────────────

    #[test]
    fn resonance_boost_exceeds_unity() {
        // With very high resonance, the signal near cutoff should exceed input RMS
        let rms_res = sine_rms_through_filter(FilterMode::LowPass, 1000.0, 0.98, 950.0, SR);
        assert!(
            rms_res > SINE_RMS,
            "High resonance near cutoff should boost above unity sine RMS ({SINE_RMS:.4}), got {rms_res:.4}"
        );
    }

    // ── Cutoff sweep ─────────────────────────────────────────────────

    #[test]
    fn cutoff_sweep_progressive_attenuation() {
        // A 5 kHz tone through progressively lower LP cutoffs should get quieter
        let cutoffs = [8000.0, 4000.0, 2000.0, 1000.0];
        let rms_values: Vec<f32> = cutoffs
            .iter()
            .map(|&c| sine_rms_through_filter(FilterMode::LowPass, c, 0.0, 5000.0, SR))
            .collect();
        for i in 1..rms_values.len() {
            assert!(
                rms_values[i] < rms_values[i - 1],
                "Lowering cutoff from {} to {} should increase attenuation of 5kHz: rms {} vs {}",
                cutoffs[i - 1],
                cutoffs[i],
                rms_values[i - 1],
                rms_values[i],
            );
        }
        // The most aggressive cutoff should produce significant attenuation
        let total_atten_db = to_db(rms_values[3] / rms_values[0]);
        assert!(
            total_atten_db < -12.0,
            "Sweeping cutoff from 8kHz to 1kHz should attenuate 5kHz by >12dB, got {total_atten_db:.1} dB"
        );
    }

    #[test]
    fn cutoff_modulation_octave_math_sanity() {
        // Verify that doubling the cutoff (1 octave up) audibly changes
        // filter behavior — this validates the octave-based modulation
        // documented on the Filter struct.
        let rms_1k = sine_rms_through_filter(FilterMode::LowPass, 1000.0, 0.0, 3000.0, SR);
        let rms_2k = sine_rms_through_filter(FilterMode::LowPass, 2000.0, 0.0, 3000.0, SR);
        // 2kHz cutoff should pass more of a 3kHz signal than 1kHz cutoff
        assert!(
            rms_2k > rms_1k,
            "doubling cutoff (1 octave) should pass more signal: 1k={rms_1k}, 2k={rms_2k}"
        );
    }

    #[test]
    fn filter_cutoff_four_octave_range() {
        // 4 octaves above 500 Hz = 500 * 2^4 = 8000 Hz.
        // A 5kHz signal should be much more audible through an 8kHz LP
        // than through a 500 Hz LP.
        let rms_base = sine_rms_through_filter(FilterMode::LowPass, 500.0, 0.0, 5000.0, SR);
        let rms_4oct_up = sine_rms_through_filter(FilterMode::LowPass, 8000.0, 0.0, 5000.0, SR);
        assert!(
            rms_4oct_up > rms_base * 2.0,
            "4 octaves of cutoff mod should dramatically change filtering: base={rms_base}, +4oct={rms_4oct_up}"
        );
    }
}
