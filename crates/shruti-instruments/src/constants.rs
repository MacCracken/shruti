//! Shared constants for the instruments crate.

// ── Musical constants ──────────────────────────────────────────────
/// A4 reference frequency in Hz (concert pitch).
pub const A4_FREQUENCY: f64 = 440.0;
/// MIDI note number for A4.
pub const A4_MIDI_NOTE: f64 = 69.0;
/// Number of semitones per octave.
pub const SEMITONES_PER_OCTAVE: f64 = 12.0;
/// Number of cents per octave (100 cents per semitone).
pub const CENTS_PER_OCTAVE: f64 = 1200.0;

// ── Filter ─────────────────────────────────────────────────────────
/// Minimum filter cutoff frequency in Hz.
pub const MIN_CUTOFF_HZ: f32 = 20.0;
/// Maximum filter cutoff frequency in Hz.
pub const MAX_CUTOFF_HZ: f32 = 20000.0;
/// Nyquist frequency multiplier (cutoff clamped to sample_rate * this).
pub const NYQUIST_RATIO: f32 = 0.49;

// ── Envelope defaults ──────────────────────────────────────────────
/// Default ADSR attack time in seconds.
pub const DEFAULT_ATTACK: f32 = 0.01;
/// Default ADSR decay time in seconds.
pub const DEFAULT_DECAY: f32 = 0.1;
/// Default ADSR sustain level (0.0–1.0).
pub const DEFAULT_SUSTAIN: f32 = 0.7;
/// Default ADSR release time in seconds.
pub const DEFAULT_RELEASE: f32 = 0.3;

// ── Polyphony ──────────────────────────────────────────────────────
/// Default maximum number of simultaneous voices.
pub const DEFAULT_MAX_VOICES: usize = 16;

// ── RNG seeds ──────────────────────────────────────────────────────
/// Initial xorshift seed for oscillator noise.
pub const OSCILLATOR_RNG_SEED: u32 = 12345;
/// Initial xorshift32 seed for LFO sample-and-hold.
pub const LFO_RNG_SEED: u32 = 0x1234_5678;

// ── Synth limits ───────────────────────────────────────────────────
/// Maximum LFO buffer frames for batch processing.
pub const MAX_LFO_FRAMES: usize = 8192;

// ── Sampler ────────────────────────────────────────────────────────
/// Hop size in samples for transient/onset detection.
pub const ONSET_HOP_SIZE: usize = 512;

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn musical_constants_correct() {
        assert!((A4_FREQUENCY - 440.0).abs() < f64::EPSILON);
        assert!((A4_MIDI_NOTE - 69.0).abs() < f64::EPSILON);
        assert!((SEMITONES_PER_OCTAVE - 12.0).abs() < f64::EPSILON);
        assert!((CENTS_PER_OCTAVE - 1200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn filter_range_valid() {
        assert!(MIN_CUTOFF_HZ > 0.0);
        assert!(MAX_CUTOFF_HZ > MIN_CUTOFF_HZ);
        assert!(NYQUIST_RATIO > 0.0 && NYQUIST_RATIO < 0.5);
    }

    #[test]
    fn envelope_defaults_valid() {
        assert!(DEFAULT_ATTACK > 0.0);
        assert!(DEFAULT_DECAY > 0.0);
        assert!((0.0..=1.0).contains(&DEFAULT_SUSTAIN));
        assert!(DEFAULT_RELEASE > 0.0);
    }

    #[test]
    fn voice_limit_nonzero() {
        assert!(DEFAULT_MAX_VOICES > 0);
    }

    #[test]
    fn rng_seeds_nonzero() {
        assert_ne!(OSCILLATOR_RNG_SEED, 0);
        assert_ne!(LFO_RNG_SEED, 0);
    }
}
