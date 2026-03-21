//! Silence detection and gain suggestion — backed by dhvani.

use crate::AudioBuffer;

/// Detect if a buffer is effectively silent (peak below threshold in dB).
pub fn is_silent(buf: &AudioBuffer, sample_rate: u32, threshold_db: f32) -> Option<bool> {
    let dbuf = dhvani::buffer::AudioBuffer::from_interleaved(
        buf.as_interleaved().to_vec(),
        buf.channels() as u32,
        sample_rate,
    )
    .ok()?;
    Some(dhvani::analysis::is_silent(&dbuf, threshold_db))
}

/// Suggest a normalization gain to reach a target RMS level.
///
/// Returns a linear gain factor clamped to 0.1–10.0. Returns 1.0 for silence.
pub fn suggest_gain(buf: &AudioBuffer, sample_rate: u32, target_rms: f32) -> Option<f32> {
    let dbuf = dhvani::buffer::AudioBuffer::from_interleaved(
        buf.as_interleaved().to_vec(),
        buf.channels() as u32,
        sample_rate,
    )
    .ok()?;
    Some(dhvani::analysis::suggest_gain(&dbuf, target_rms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_detected() {
        let buf = AudioBuffer::new(1, 1024);
        assert!(is_silent(&buf, 48000, -60.0).unwrap());
    }

    #[test]
    fn signal_not_silent() {
        let mut buf = AudioBuffer::new(1, 1024);
        for i in 0..1024 {
            buf.set(i, 0, 0.5);
        }
        assert!(!is_silent(&buf, 48000, -60.0).unwrap());
    }

    #[test]
    fn suggest_gain_for_quiet() {
        let mut buf = AudioBuffer::new(1, 1024);
        for i in 0..1024 {
            buf.set(i, 0, 0.01);
        }
        let gain = suggest_gain(&buf, 48000, 0.125).unwrap();
        assert!(gain > 1.0, "quiet signal should get gain > 1.0, got {gain}");
    }

    #[test]
    fn suggest_gain_for_silence() {
        let buf = AudioBuffer::new(1, 1024);
        let gain = suggest_gain(&buf, 48000, 0.125).unwrap();
        assert_eq!(gain, 1.0, "silence should return 1.0");
    }
}
