//! Onset detection — transient boundaries via spectral flux, backed by dhvani.

pub use dhvani::analysis::OnsetResult;

use crate::AudioBuffer;

/// Detect note/transient onsets in an audio buffer.
///
/// - `window_size`: FFT window size (power of 2, e.g., 2048)
/// - `hop_size`: hop between windows (e.g., 512)
/// - `threshold`: onset sensitivity (0.0–1.0, lower = more sensitive)
pub fn detect_onsets(
    buf: &AudioBuffer,
    sample_rate: u32,
    window_size: usize,
    hop_size: usize,
    threshold: f32,
) -> Option<OnsetResult> {
    let dbuf = dhvani::buffer::AudioBuffer::from_interleaved(
        buf.as_interleaved().to_vec(),
        buf.channels() as u32,
        sample_rate,
    )
    .ok()?;
    Some(dhvani::analysis::detect_onsets(
        &dbuf,
        window_size,
        hop_size,
        threshold,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_no_onsets() {
        let buf = AudioBuffer::new(1, 4096);
        let result = detect_onsets(&buf, 48000, 2048, 512, 0.3).unwrap();
        assert!(result.positions.is_empty());
    }

    #[test]
    fn impulse_detected() {
        let sr = 48000u32;
        let n = sr * 2; // 2 seconds
        let mut buf = AudioBuffer::new(1, n);
        // Create silence, then a sudden loud sine burst at 0.5s
        let onset_frame = sr / 2;
        for i in onset_frame..onset_frame + sr / 4 {
            let s = (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / sr as f64).sin() as f32 * 0.9;
            buf.set(i, 0, s);
        }
        let result = detect_onsets(&buf, sr, 2048, 512, 0.1).unwrap();
        // Should detect at least one onset
        assert!(
            !result.positions.is_empty(),
            "should detect onset from burst"
        );
    }
}
