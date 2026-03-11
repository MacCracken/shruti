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
pub struct Filter {
    pub mode: FilterMode,
    pub cutoff: f32,
    pub resonance: f32,
    sample_rate: f32,
    // SVF state variables
    ic1eq: f32,
    ic2eq: f32,
}

impl Filter {
    pub fn new(mode: FilterMode, cutoff: f32, resonance: f32, sample_rate: f32) -> Self {
        Self {
            mode,
            cutoff: cutoff.clamp(20.0, 20000.0),
            resonance: resonance.clamp(0.0, 1.0),
            sample_rate,
            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr;
    }

    /// Process a single sample through the SVF.
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let g = (std::f32::consts::PI * self.cutoff / self.sample_rate).tan();
        let k = 2.0 - 2.0 * self.resonance;

        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;

        let v3 = input - self.ic2eq;
        let v1 = a1 * self.ic1eq + a2 * v3;
        let v2 = self.ic2eq + a2 * self.ic1eq + a3 * v3;

        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        match self.mode {
            FilterMode::LowPass => v2,
            FilterMode::HighPass => input - k * v1 - v2,
            FilterMode::BandPass => v1,
            FilterMode::Notch => input - k * v1,
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
}
