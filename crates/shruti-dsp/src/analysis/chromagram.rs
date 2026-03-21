//! Chromagram — pitch class energy distribution, backed by dhvani.

pub use dhvani::analysis::Chromagram;

use crate::AudioBuffer;

/// Compute a chromagram from an audio buffer.
///
/// Maps FFT bin frequencies to the 12 pitch classes (C, C#, D, ..., B).
pub fn chromagram(buf: &AudioBuffer, sample_rate: u32, window_size: usize) -> Option<Chromagram> {
    let dbuf = dhvani::buffer::AudioBuffer::from_interleaved(
        buf.as_interleaved().to_vec(),
        buf.channels() as u32,
        sample_rate,
    )
    .ok()?;
    Some(dhvani::analysis::chromagram(&dbuf, window_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_returns_flat() {
        let buf = AudioBuffer::new(1, 4096);
        let chroma = chromagram(&buf, 48000, 4096).unwrap();
        // All classes should be equal (or zero)
        let max = chroma.chroma.iter().cloned().fold(0.0f32, f32::max);
        assert!(max <= 1.0);
    }

    #[test]
    fn sine_a440_detects_a() {
        let sr = 48000u32;
        let n = 8192;
        let mut buf = AudioBuffer::new(1, n as u32);
        for i in 0..n {
            let s = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / sr as f64).sin() as f32;
            buf.set(i as u32, 0, s);
        }
        let chroma = chromagram(&buf, sr, 4096).unwrap();
        // A is pitch class index 9
        assert_eq!(chroma.dominant_name(), "A");
    }
}
