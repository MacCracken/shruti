//! Loudness measurement and normalization via tarang.
//!
//! Available only with the `tarang` feature. Wraps tarang's BS.1770-based
//! loudness analysis with shruti's `AudioBuffer` types.

use crate::buffer::AudioBuffer;
use crate::format::AudioFormat;

/// Loudness measurement results.
#[derive(Debug, Clone, Copy)]
pub struct LoudnessMetrics {
    /// Integrated loudness in LUFS (negative; typical music: -14 to -8).
    pub integrated_lufs: f64,
    /// Peak sample value (0.0-1.0).
    pub peak: f32,
    /// RMS level in dB (relative to full scale).
    pub rms_db: f64,
}

/// Convert a shruti AudioBuffer to a tarang AudioBuffer.
fn to_tarang(buffer: &AudioBuffer, format: &AudioFormat) -> tarang::core::AudioBuffer {
    let interleaved = buffer.as_interleaved();
    let byte_data: Vec<u8> = interleaved.iter().flat_map(|s| s.to_le_bytes()).collect();
    tarang::core::AudioBuffer {
        data: bytes::Bytes::from(byte_data),
        sample_format: tarang::core::SampleFormat::F32,
        channels: {
            debug_assert!(buffer.channels() > 0, "AudioBuffer has 0 channels");
            buffer.channels().max(1)
        },
        sample_rate: format.sample_rate,
        num_frames: buffer.frames() as usize,
        timestamp: std::time::Duration::ZERO,
    }
}

/// Convert a tarang AudioBuffer back to a shruti AudioBuffer.
fn from_tarang(tarang_buf: &tarang::core::AudioBuffer) -> AudioBuffer {
    let float_bytes = &tarang_buf.data;
    let num_floats = float_bytes.len() / 4;
    let mut samples = Vec::with_capacity(num_floats);
    for chunk in float_bytes.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    AudioBuffer::from_interleaved(samples, tarang_buf.channels)
}

/// Measure the loudness of an audio buffer.
///
/// Uses simplified ITU-R BS.1770 measurement via tarang.
pub fn measure_loudness(buffer: &AudioBuffer, format: &AudioFormat) -> LoudnessMetrics {
    let tarang_buf = to_tarang(buffer, format);
    let m = tarang::audio::loudness::measure_loudness(&tarang_buf);
    LoudnessMetrics {
        integrated_lufs: m.integrated_lufs,
        peak: m.peak,
        rms_db: m.rms_db,
    }
}

/// Apply a gain adjustment in decibels to an audio buffer.
///
/// Positive dB = louder, negative = quieter. Samples are clamped to [-1.0, 1.0].
pub fn apply_gain(
    buffer: &AudioBuffer,
    format: &AudioFormat,
    gain_db: f32,
) -> Result<AudioBuffer, Box<dyn std::error::Error>> {
    let tarang_buf = to_tarang(buffer, format);
    let result = tarang::audio::loudness::apply_gain(&tarang_buf, gain_db)?;
    Ok(from_tarang(&result))
}

/// Normalize an audio buffer to a target loudness in LUFS.
///
/// Common targets: -14 LUFS (Spotify/YouTube), -16 LUFS (podcast).
/// Returns an error if the audio is too quiet (>40dB gain needed).
pub fn normalize_loudness(
    buffer: &AudioBuffer,
    format: &AudioFormat,
    target_lufs: f64,
) -> Result<AudioBuffer, Box<dyn std::error::Error>> {
    let tarang_buf = to_tarang(buffer, format);
    let result = tarang::audio::loudness::normalize_loudness(&tarang_buf, target_lufs)?;
    Ok(from_tarang(&result))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(
        sample_rate: u32,
        channels: u16,
        duration_secs: f32,
        amplitude: f32,
    ) -> (AudioBuffer, AudioFormat) {
        let num_frames = (sample_rate as f32 * duration_secs) as usize;
        let mut samples = Vec::with_capacity(num_frames * channels as usize);
        for i in 0..num_frames {
            let t = i as f32 / sample_rate as f32;
            let s = amplitude * (t * 440.0 * std::f32::consts::TAU).sin();
            for _ in 0..channels {
                samples.push(s);
            }
        }
        let buf = AudioBuffer::from_interleaved(samples, channels);
        let fmt = AudioFormat::new(sample_rate, channels, 0);
        (buf, fmt)
    }

    #[test]
    fn silence_is_quiet() {
        let buf = AudioBuffer::new(1, 0);
        let fmt = AudioFormat::new(44100, 1, 0);
        let m = measure_loudness(&buf, &fmt);
        assert!(m.integrated_lufs < -60.0);
    }

    #[test]
    fn loud_audio_has_higher_lufs() {
        let (loud, fmt) = make_sine(44100, 1, 1.0, 0.9);
        let (quiet, _) = make_sine(44100, 1, 1.0, 0.1);
        let m_loud = measure_loudness(&loud, &fmt);
        let m_quiet = measure_loudness(&quiet, &fmt);
        assert!(m_loud.integrated_lufs > m_quiet.integrated_lufs);
    }

    #[test]
    fn apply_gain_increases_level() {
        let (buf, fmt) = make_sine(44100, 1, 0.5, 0.3);
        let before = measure_loudness(&buf, &fmt);
        let boosted = apply_gain(&buf, &fmt, 6.0).unwrap();
        let after = measure_loudness(&boosted, &fmt);
        assert!(after.integrated_lufs > before.integrated_lufs);
    }

    #[test]
    fn normalize_moves_toward_target() {
        let (buf, fmt) = make_sine(44100, 1, 1.0, 0.1);
        let before = measure_loudness(&buf, &fmt);
        let target = -14.0;

        let normalized = normalize_loudness(&buf, &fmt, target).unwrap();
        let after = measure_loudness(&normalized, &fmt);

        assert!(
            (after.integrated_lufs - target).abs() < (before.integrated_lufs - target).abs(),
            "normalized ({:.1}) should be closer to target ({target}) than before ({:.1})",
            after.integrated_lufs,
            before.integrated_lufs,
        );
    }

    #[test]
    fn normalize_silence_errors() {
        let buf = AudioBuffer::from_interleaved(vec![0.0; 1000], 1);
        let fmt = AudioFormat::new(44100, 1, 0);
        let result = normalize_loudness(&buf, &fmt, -14.0);
        assert!(result.is_err());
    }
}
