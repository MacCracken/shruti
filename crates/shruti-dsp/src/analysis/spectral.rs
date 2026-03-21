//! Spectral analysis — backed by dhvani's FFT.

use crate::AudioBuffer;
use crate::constants::{MAX_FFT_SIZE, SPECTRAL_ROLLOFF_THRESHOLD};

/// Result of spectral analysis on a buffer.
#[derive(Debug, Clone)]
pub struct SpectralAnalysis {
    /// Magnitude spectrum in dB (frequency bins from 0 to Nyquist).
    pub magnitude_db: Vec<f32>,
    /// Frequency resolution (Hz per bin).
    pub frequency_resolution: f32,
    /// Sample rate used for analysis.
    pub sample_rate: u32,
    /// FFT size used.
    pub fft_size: usize,
    /// Peak frequency in Hz.
    pub peak_frequency: f32,
    /// Peak magnitude in dB.
    pub peak_magnitude_db: f32,
    /// Spectral centroid in Hz (brightness indicator).
    pub spectral_centroid: f32,
    /// Spectral rolloff frequency in Hz (95% energy threshold).
    pub spectral_rolloff: f32,
}

/// Perform spectral analysis on an AudioBuffer.
/// Analyzes the specified channel (default 0).
/// fft_size must be a power of 2 and <= 65536; the buffer is zero-padded or truncated as needed.
/// Returns None if fft_size is invalid (not power of 2 or exceeds maximum).
pub fn analyze_spectrum(
    buffer: &AudioBuffer,
    channel: u16,
    sample_rate: u32,
    fft_size: usize,
) -> Option<SpectralAnalysis> {
    if !fft_size.is_power_of_two() || fft_size == 0 || fft_size > MAX_FFT_SIZE {
        return None;
    }

    // Extract samples from the specified channel
    let frames = buffer.frames() as usize;
    let n = fft_size.min(frames);
    let mut mono_samples = vec![0.0f32; fft_size];
    for (i, sample) in mono_samples.iter_mut().enumerate().take(n) {
        *sample = buffer.get(i as u32, channel);
    }

    // Create a mono dhvani buffer and run FFT
    let dbuf = dhvani::buffer::AudioBuffer::from_interleaved(mono_samples, 1, sample_rate).ok()?;
    let spectrum = dhvani::analysis::spectrum_fft(&dbuf, fft_size);

    let centroid = spectrum.spectral_centroid();
    let rolloff = spectrum.spectral_rolloff(SPECTRAL_ROLLOFF_THRESHOLD);

    Some(SpectralAnalysis {
        magnitude_db: spectrum.magnitude_db,
        frequency_resolution: spectrum.freq_resolution,
        sample_rate: spectrum.sample_rate,
        fft_size: spectrum.fft_size,
        peak_frequency: spectrum.peak_frequency,
        peak_magnitude_db: spectrum.peak_magnitude_db,
        spectral_centroid: centroid,
        spectral_rolloff: rolloff,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AudioBuffer;

    #[test]
    fn silence_produces_low_magnitudes() {
        let buf = AudioBuffer::new(1, 1024);
        let result = analyze_spectrum(&buf, 0, 48000, 1024).unwrap();
        assert_eq!(result.fft_size, 1024);
        for &db in &result.magnitude_db {
            assert!(db < -100.0);
        }
    }

    #[test]
    fn sine_wave_peak_detection() {
        let sr = 48000u32;
        let fft_size = 4096;
        let freq = 1000.0f32;
        let mut buf = AudioBuffer::new(1, fft_size as u32);
        for i in 0..fft_size {
            let sample =
                (2.0 * std::f64::consts::PI * freq as f64 * i as f64 / sr as f64).sin() as f32;
            buf.set(i as u32, 0, sample);
        }
        let result = analyze_spectrum(&buf, 0, sr, fft_size).unwrap();
        let bin_hz = result.frequency_resolution;
        assert!(
            (result.peak_frequency - freq).abs() < bin_hz * 2.0,
            "peak at {} Hz, expected near {} Hz",
            result.peak_frequency,
            freq
        );
    }

    #[test]
    fn spectral_centroid_of_sine() {
        let sr = 48000u32;
        let fft_size = 4096;
        let freq = 2000.0f32;
        let mut buf = AudioBuffer::new(1, fft_size as u32);
        for i in 0..fft_size {
            let sample =
                (2.0 * std::f64::consts::PI * freq as f64 * i as f64 / sr as f64).sin() as f32;
            buf.set(i as u32, 0, sample);
        }
        let result = analyze_spectrum(&buf, 0, sr, fft_size).unwrap();
        assert!(
            (result.spectral_centroid - freq).abs() < 200.0,
            "centroid at {} Hz, expected near {} Hz",
            result.spectral_centroid,
            freq
        );
    }

    #[test]
    fn frequency_resolution_is_correct() {
        let buf = AudioBuffer::new(1, 1024);
        let result = analyze_spectrum(&buf, 0, 44100, 1024).unwrap();
        let expected = 44100.0 / 1024.0;
        assert!((result.frequency_resolution - expected).abs() < 0.1);
    }

    #[test]
    fn invalid_fft_size_returns_none() {
        let buf = AudioBuffer::new(1, 1024);
        assert!(analyze_spectrum(&buf, 0, 48000, 0).is_none());
        assert!(analyze_spectrum(&buf, 0, 48000, 100).is_none());
        assert!(analyze_spectrum(&buf, 0, 48000, 131072).is_none());
    }
}
