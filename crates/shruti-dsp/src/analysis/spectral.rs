use crate::AudioBuffer;

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
/// fft_size must be a power of 2; the buffer is zero-padded or truncated as needed.
pub fn analyze_spectrum(
    buffer: &AudioBuffer,
    channel: u16,
    sample_rate: u32,
    fft_size: usize,
) -> SpectralAnalysis {
    assert!(fft_size.is_power_of_two(), "FFT size must be a power of 2");

    // Extract samples from the specified channel
    let frames = buffer.frames() as usize;
    let n = fft_size.min(frames);

    let mut real = vec![0.0f64; fft_size];
    let mut imag = vec![0.0f64; fft_size];

    // Copy samples and apply Hann window
    for (i, real_val) in real.iter_mut().enumerate().take(n) {
        let window = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (n as f64 - 1.0)).cos());
        *real_val = buffer.get(i as u32, channel) as f64 * window;
    }

    // In-place radix-2 FFT
    fft_radix2(&mut real, &mut imag);

    // Compute magnitude spectrum (only positive frequencies: bins 0..fft_size/2+1)
    let num_bins = fft_size / 2 + 1;
    let mut magnitude_db = Vec::with_capacity(num_bins);
    let mut magnitudes_linear = Vec::with_capacity(num_bins);

    for i in 0..num_bins {
        let mag = (real[i] * real[i] + imag[i] * imag[i]).sqrt() as f32;
        magnitudes_linear.push(mag);
        let db = if mag > 1e-10 {
            20.0 * mag.log10()
        } else {
            -200.0
        };
        magnitude_db.push(db);
    }

    let frequency_resolution = sample_rate as f32 / fft_size as f32;

    // Peak frequency
    let (peak_bin, &peak_mag) = magnitudes_linear
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, &0.0));
    let peak_frequency = peak_bin as f32 * frequency_resolution;
    let peak_magnitude_db = if peak_mag > 1e-10 {
        20.0 * peak_mag.log10()
    } else {
        -200.0
    };

    // Spectral centroid: weighted mean of frequencies by magnitude
    let total_mag: f32 = magnitudes_linear.iter().sum();
    let spectral_centroid = if total_mag > 1e-10 {
        magnitudes_linear
            .iter()
            .enumerate()
            .map(|(i, &m)| i as f32 * frequency_resolution * m)
            .sum::<f32>()
            / total_mag
    } else {
        0.0
    };

    // Spectral rolloff: frequency below which 95% of spectral energy is concentrated
    let total_energy: f32 = magnitudes_linear.iter().map(|m| m * m).sum();
    let threshold = total_energy * 0.95;
    let mut cumulative = 0.0f32;
    let mut rolloff_bin = num_bins - 1;
    for (i, &m) in magnitudes_linear.iter().enumerate() {
        cumulative += m * m;
        if cumulative >= threshold {
            rolloff_bin = i;
            break;
        }
    }
    let spectral_rolloff = rolloff_bin as f32 * frequency_resolution;

    SpectralAnalysis {
        magnitude_db,
        frequency_resolution,
        sample_rate,
        fft_size,
        peak_frequency,
        peak_magnitude_db,
        spectral_centroid,
        spectral_rolloff,
    }
}

/// In-place radix-2 Cooley-Tukey FFT.
fn fft_radix2(real: &mut [f64], imag: &mut [f64]) {
    let n = real.len();
    assert_eq!(n, imag.len());
    assert!(n.is_power_of_two());

    // Bit-reversal permutation
    let mut j = 0;
    for i in 0..n {
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
        let mut m = n >> 1;
        while m >= 1 && j >= m {
            j -= m;
            m >>= 1;
        }
        j += m;
    }

    // Butterfly operations
    let mut size = 2;
    while size <= n {
        let half = size / 2;
        let angle = -2.0 * std::f64::consts::PI / size as f64;

        let wn_r = angle.cos();
        let wn_i = angle.sin();

        let mut i = 0;
        while i < n {
            let mut w_r = 1.0;
            let mut w_i = 0.0;

            for k in 0..half {
                let even = i + k;
                let odd = i + k + half;

                let tr = w_r * real[odd] - w_i * imag[odd];
                let ti = w_r * imag[odd] + w_i * real[odd];

                real[odd] = real[even] - tr;
                imag[odd] = imag[even] - ti;
                real[even] += tr;
                imag[even] += ti;

                let new_w_r = w_r * wn_r - w_i * wn_i;
                let new_w_i = w_r * wn_i + w_i * wn_r;
                w_r = new_w_r;
                w_i = new_w_i;
            }

            i += size;
        }

        size *= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AudioBuffer;

    #[test]
    fn silence_produces_low_magnitudes() {
        let buf = AudioBuffer::new(1, 1024);
        let result = analyze_spectrum(&buf, 0, 48000, 1024);
        assert_eq!(result.fft_size, 1024);
        assert_eq!(result.magnitude_db.len(), 513); // fft_size/2 + 1
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
        let result = analyze_spectrum(&buf, 0, sr, fft_size);
        // Peak should be near 1000 Hz (within one bin)
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
        let result = analyze_spectrum(&buf, 0, sr, fft_size);
        // Centroid should be near the sine frequency
        assert!(
            (result.spectral_centroid - freq).abs() < 200.0,
            "centroid at {} Hz, expected near {} Hz",
            result.spectral_centroid,
            freq
        );
    }

    #[test]
    fn fft_roundtrip_dc() {
        let mut real = vec![1.0; 8];
        let mut imag = vec![0.0; 8];
        fft_radix2(&mut real, &mut imag);
        // DC bin should have magnitude 8 (sum of all 1s)
        assert!((real[0] - 8.0).abs() < 1e-10);
        // All other bins should be ~0 for constant signal
        for i in 1..8 {
            let mag = (real[i] * real[i] + imag[i] * imag[i]).sqrt();
            assert!(mag < 1e-10, "bin {} has magnitude {}", i, mag);
        }
    }

    #[test]
    fn frequency_resolution_is_correct() {
        let buf = AudioBuffer::new(1, 1024);
        let result = analyze_spectrum(&buf, 0, 44100, 1024);
        let expected = 44100.0 / 1024.0;
        assert!((result.frequency_resolution - expected).abs() < 0.01);
    }
}
