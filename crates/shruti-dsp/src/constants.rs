//! Shared audio constants for the DSP crate.

// ── dB / level conversion ──────────────────────────────────────────
/// Floor value for dB conversion — represents silence.
pub const DB_FLOOR: f32 = -200.0;
/// Minimum linear amplitude treated as non-zero for dB conversion.
pub const LINEAR_FLOOR: f32 = 1e-10;
/// Minimum linear amplitude (f64) for LUFS calculation.
pub const LINEAR_FLOOR_F64: f64 = 1e-20;
/// EBU R128 LUFS offset constant (-0.691 dB).
pub const LUFS_OFFSET: f64 = -0.691;

// ── Reverb (Freeverb) ──────────────────────────────────────────────
/// Reference sample rate for Freeverb filter length tuning.
pub const FREEVERB_REFERENCE_RATE: f32 = 44100.0;
/// Allpass filter feedback coefficient.
pub const ALLPASS_FEEDBACK: f32 = 0.5;
/// Default reverb dry/wet mix.
pub const DEFAULT_REVERB_MIX: f32 = 0.3;
/// Default reverb room size.
pub const DEFAULT_REVERB_ROOM_SIZE: f32 = 0.7;
/// Default reverb damping.
pub const DEFAULT_REVERB_DAMPING: f32 = 0.5;
/// Feedback base value for room size calculation.
pub const REVERB_FEEDBACK_BASE: f32 = 0.7;
/// Feedback scale factor for room size calculation.
pub const REVERB_FEEDBACK_SCALE: f32 = 0.28;
/// Damping scale factor.
pub const REVERB_DAMPING_SCALE: f32 = 0.4;

// ── Delay ──────────────────────────────────────────────────────────
/// Maximum delay time in seconds.
pub const MAX_DELAY_SECONDS: f32 = 5.0;
/// Default delay time in seconds.
pub const DEFAULT_DELAY_TIME: f32 = 0.25;
/// Default delay feedback amount.
pub const DEFAULT_DELAY_FEEDBACK: f32 = 0.4;
/// Default delay dry/wet mix.
pub const DEFAULT_DELAY_MIX: f32 = 0.3;

// ── Compressor ─────────────────────────────────────────────────────
/// Default compressor threshold in dB.
pub const DEFAULT_COMPRESSOR_THRESHOLD: f32 = -20.0;
/// Default compression ratio.
pub const DEFAULT_COMPRESSOR_RATIO: f32 = 4.0;
/// Default compressor attack time in seconds.
pub const DEFAULT_COMPRESSOR_ATTACK: f32 = 0.01;
/// Default compressor release time in seconds.
pub const DEFAULT_COMPRESSOR_RELEASE: f32 = 0.1;
/// Default compressor knee width in dB.
pub const DEFAULT_COMPRESSOR_KNEE: f32 = 6.0;

// ── Limiter ────────────────────────────────────────────────────────
/// Default limiter ceiling in dB.
pub const DEFAULT_LIMITER_CEILING: f32 = -0.3;
/// Default limiter release time in seconds.
pub const DEFAULT_LIMITER_RELEASE: f32 = 0.1;

// ── Meter ──────────────────────────────────────────────────────────
/// LUFS block duration in seconds (EBU R128: 400ms).
pub const LUFS_BLOCK_DURATION: f32 = 0.4;
/// Peak hold decay coefficient per sample.
pub const PEAK_DECAY_COEFFICIENT: f32 = 0.9995;
/// Maximum number of LUFS blocks retained (~10 minutes of 400ms blocks).
pub const MAX_LUFS_BLOCKS: usize = 1500;

// ── De-Esser ─────────────────────────────────────────────────────
/// Default de-esser center frequency in Hz.
pub const DEFAULT_DEESSER_FREQ_HZ: f32 = 6000.0;
/// Default de-esser threshold in dB.
pub const DEFAULT_DEESSER_THRESHOLD_DB: f32 = -20.0;
/// Default de-esser reduction in dB.
pub const DEFAULT_DEESSER_REDUCTION_DB: f32 = 6.0;
/// Default de-esser Q factor.
pub const DEFAULT_DEESSER_Q: f32 = 2.0;

// ── LFO ──────────────────────────────────────────────────────────
/// Default LFO rate in Hz.
pub const DEFAULT_LFO_RATE: f32 = 1.0;
/// Default LFO depth (0.0–1.0).
pub const DEFAULT_LFO_DEPTH: f32 = 0.5;

// ── Envelope (ADSR) ──────────────────────────────────────────────
/// Default envelope attack time in seconds.
pub const DEFAULT_ENVELOPE_ATTACK: f32 = 0.01;
/// Default envelope decay time in seconds.
pub const DEFAULT_ENVELOPE_DECAY: f32 = 0.1;
/// Default envelope sustain level (0.0–1.0).
pub const DEFAULT_ENVELOPE_SUSTAIN: f32 = 0.7;
/// Default envelope release time in seconds.
pub const DEFAULT_ENVELOPE_RELEASE: f32 = 0.3;

// ── Gain Smoother ────────────────────────────────────────────────
/// Default gain smoother attack coefficient.
pub const DEFAULT_GAIN_SMOOTHER_ATTACK: f32 = 0.3;
/// Default gain smoother release coefficient.
pub const DEFAULT_GAIN_SMOOTHER_RELEASE: f32 = 0.05;

// ── Modulated Delay ──────────────────────────────────────────────
/// Default modulated delay base delay in ms.
pub const DEFAULT_MOD_DELAY_BASE_MS: f32 = 7.0;
/// Default modulated delay depth in ms.
pub const DEFAULT_MOD_DELAY_DEPTH_MS: f32 = 3.0;
/// Default modulated delay rate in Hz.
pub const DEFAULT_MOD_DELAY_RATE_HZ: f32 = 0.5;
/// Default modulated delay feedback.
pub const DEFAULT_MOD_DELAY_FEEDBACK: f32 = 0.3;
/// Default modulated delay mix.
pub const DEFAULT_MOD_DELAY_MIX: f32 = 0.5;

// ── Spectral analysis ──────────────────────────────────────────────
/// Maximum allowed FFT size to prevent excessive memory allocation.
pub const MAX_FFT_SIZE: usize = 65536;
/// Spectral rolloff energy threshold (95%).
pub const SPECTRAL_ROLLOFF_THRESHOLD: f32 = 0.95;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn constants_are_sane() {
        assert!(DB_FLOOR < -100.0);
        assert!(LINEAR_FLOOR > 0.0);
        assert!(LINEAR_FLOOR < 1e-5);
        assert!(LUFS_OFFSET < 0.0);
        assert!(MAX_DELAY_SECONDS > 0.0);
        assert!(DEFAULT_COMPRESSOR_RATIO > 1.0);
        assert!(DEFAULT_LIMITER_CEILING < 0.0);
        assert!(LUFS_BLOCK_DURATION > 0.0);
        assert!(PEAK_DECAY_COEFFICIENT > 0.0 && PEAK_DECAY_COEFFICIENT < 1.0);
        assert!(MAX_FFT_SIZE.is_power_of_two());
        assert!(SPECTRAL_ROLLOFF_THRESHOLD > 0.0 && SPECTRAL_ROLLOFF_THRESHOLD < 1.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn reverb_defaults_in_range() {
        assert!((0.0..=1.0).contains(&DEFAULT_REVERB_MIX));
        assert!((0.0..=1.0).contains(&DEFAULT_REVERB_ROOM_SIZE));
        assert!((0.0..=1.0).contains(&DEFAULT_REVERB_DAMPING));
        assert!(FREEVERB_REFERENCE_RATE > 0.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn delay_defaults_in_range() {
        assert!(DEFAULT_DELAY_TIME > 0.0 && DEFAULT_DELAY_TIME <= MAX_DELAY_SECONDS);
        assert!((0.0..1.0).contains(&DEFAULT_DELAY_FEEDBACK));
        assert!((0.0..=1.0).contains(&DEFAULT_DELAY_MIX));
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn compressor_defaults_reasonable() {
        assert!(DEFAULT_COMPRESSOR_THRESHOLD < 0.0);
        assert!(DEFAULT_COMPRESSOR_ATTACK > 0.0);
        assert!(DEFAULT_COMPRESSOR_RELEASE > 0.0);
        assert!(DEFAULT_COMPRESSOR_KNEE >= 0.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn deesser_defaults_reasonable() {
        assert!(DEFAULT_DEESSER_FREQ_HZ > 1000.0);
        assert!(DEFAULT_DEESSER_THRESHOLD_DB < 0.0);
        assert!(DEFAULT_DEESSER_REDUCTION_DB > 0.0);
        assert!(DEFAULT_DEESSER_Q > 0.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn lfo_defaults_reasonable() {
        assert!(DEFAULT_LFO_RATE > 0.0);
        assert!((0.0..=1.0).contains(&DEFAULT_LFO_DEPTH));
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn envelope_defaults_reasonable() {
        assert!(DEFAULT_ENVELOPE_ATTACK > 0.0);
        assert!(DEFAULT_ENVELOPE_DECAY > 0.0);
        assert!((0.0..=1.0).contains(&DEFAULT_ENVELOPE_SUSTAIN));
        assert!(DEFAULT_ENVELOPE_RELEASE > 0.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn gain_smoother_defaults_reasonable() {
        assert!((0.0..=1.0).contains(&DEFAULT_GAIN_SMOOTHER_ATTACK));
        assert!((0.0..=1.0).contains(&DEFAULT_GAIN_SMOOTHER_RELEASE));
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn mod_delay_defaults_reasonable() {
        assert!(DEFAULT_MOD_DELAY_BASE_MS > 0.0);
        assert!(DEFAULT_MOD_DELAY_DEPTH_MS > 0.0);
        assert!(DEFAULT_MOD_DELAY_RATE_HZ > 0.0);
        assert!((0.0..1.0).contains(&DEFAULT_MOD_DELAY_FEEDBACK));
        assert!((0.0..=1.0).contains(&DEFAULT_MOD_DELAY_MIX));
    }
}
