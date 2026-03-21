//! Oscillator — anti-aliased waveform generation backed by dhvani.

pub use dhvani::dsp::Waveform;

/// Anti-aliased oscillator with PolyBLEP correction.
#[derive(Debug, Clone)]
pub struct Oscillator {
    sample_rate: f32,
    inner: dhvani::dsp::Oscillator,
}

impl Oscillator {
    pub fn new(waveform: Waveform, sample_rate: f32) -> Self {
        Self {
            sample_rate,
            inner: dhvani::dsp::Oscillator::new(waveform, sample_rate as u32),
        }
    }

    /// Generate the next sample at the given frequency. Call once per sample.
    pub fn sample(&mut self, freq: f64) -> f32 {
        self.inner.sample(freq)
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.inner.set_waveform(waveform);
    }

    pub fn waveform(&self) -> Waveform {
        self.inner.waveform()
    }

    pub fn phase(&self) -> f64 {
        self.inner.phase()
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.inner.set_sample_rate(sample_rate as u32);
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_produces_signal() {
        let mut osc = Oscillator::new(Waveform::Sine, 48000.0);
        let mut has_nonzero = false;
        for _ in 0..480 {
            let s = osc.sample(440.0);
            if s.abs() > 0.01 {
                has_nonzero = true;
            }
        }
        assert!(has_nonzero);
    }

    #[test]
    fn output_in_range() {
        let mut osc = Oscillator::new(Waveform::Square, 48000.0);
        for _ in 0..4800 {
            let s = osc.sample(440.0);
            assert!((-1.1..=1.1).contains(&s), "sample out of range: {s}");
        }
    }

    #[test]
    fn different_waveforms() {
        for wf in [
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Triangle,
        ] {
            let mut osc = Oscillator::new(wf, 48000.0);
            let s = osc.sample(440.0);
            assert!(s.is_finite());
        }
    }

    #[test]
    fn set_waveform_works() {
        let mut osc = Oscillator::new(Waveform::Sine, 48000.0);
        assert_eq!(osc.waveform(), Waveform::Sine);
        osc.set_waveform(Waveform::Saw);
        assert_eq!(osc.waveform(), Waveform::Saw);
    }

    #[test]
    fn reset_clears_phase() {
        let mut osc = Oscillator::new(Waveform::Sine, 48000.0);
        osc.sample(440.0);
        assert!(osc.phase() > 0.0);
        osc.reset();
        assert_eq!(osc.phase(), 0.0);
    }
}
