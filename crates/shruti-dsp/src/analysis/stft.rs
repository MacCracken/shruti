//! STFT — Short-Time Fourier Transform for spectrogram generation, backed by dhvani.

pub use dhvani::analysis::Spectrogram;

use crate::AudioBuffer;

/// Compute a spectrogram using STFT.
///
/// - `window_size`: FFT window size (power of 2)
/// - `hop_size`: hop between windows
pub fn compute_stft(
    buf: &AudioBuffer,
    sample_rate: u32,
    window_size: usize,
    hop_size: usize,
) -> Option<Spectrogram> {
    let dbuf = dhvani::buffer::AudioBuffer::from_interleaved(
        buf.as_interleaved().to_vec(),
        buf.channels() as u32,
        sample_rate,
    )
    .ok()?;
    Some(dhvani::analysis::compute_stft(&dbuf, window_size, hop_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_produces_spectrogram() {
        let buf = AudioBuffer::new(1, 4096);
        let sg = compute_stft(&buf, 48000, 1024, 512).unwrap();
        assert!(sg.num_frames() > 0);
        assert!(sg.num_bins > 0);
    }

    #[test]
    fn spectrogram_dimensions() {
        let buf = AudioBuffer::new(1, 8192);
        let window = 1024;
        let hop = 512;
        let sg = compute_stft(&buf, 48000, window, hop).unwrap();
        assert_eq!(sg.num_bins, window / 2);
        // Expected frames: (8192 - 1024) / 512 + 1 = ~14
        assert!(sg.num_frames() >= 10);
    }

    #[test]
    fn freq_resolution_correct() {
        let buf = AudioBuffer::new(1, 4096);
        let sg = compute_stft(&buf, 44100, 1024, 512).unwrap();
        let expected = 44100.0 / 1024.0;
        assert!((sg.freq_resolution - expected).abs() < 0.1);
    }
}
