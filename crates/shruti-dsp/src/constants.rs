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

// ── Spectral analysis ──────────────────────────────────────────────
/// Maximum allowed FFT size to prevent excessive memory allocation.
pub const MAX_FFT_SIZE: usize = 65536;
/// Spectral rolloff energy threshold (95%).
pub const SPECTRAL_ROLLOFF_THRESHOLD: f32 = 0.95;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
    fn reverb_defaults_in_range() {
        assert!((0.0..=1.0).contains(&DEFAULT_REVERB_MIX));
        assert!((0.0..=1.0).contains(&DEFAULT_REVERB_ROOM_SIZE));
        assert!((0.0..=1.0).contains(&DEFAULT_REVERB_DAMPING));
        assert!(FREEVERB_REFERENCE_RATE > 0.0);
    }

    #[test]
    fn delay_defaults_in_range() {
        assert!(DEFAULT_DELAY_TIME > 0.0 && DEFAULT_DELAY_TIME <= MAX_DELAY_SECONDS);
        assert!((0.0..1.0).contains(&DEFAULT_DELAY_FEEDBACK));
        assert!((0.0..=1.0).contains(&DEFAULT_DELAY_MIX));
    }

    #[test]
    fn compressor_defaults_reasonable() {
        assert!(DEFAULT_COMPRESSOR_THRESHOLD < 0.0);
        assert!(DEFAULT_COMPRESSOR_ATTACK > 0.0);
        assert!(DEFAULT_COMPRESSOR_RELEASE > 0.0);
        assert!(DEFAULT_COMPRESSOR_KNEE >= 0.0);
    }
}
