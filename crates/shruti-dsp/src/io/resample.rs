//! Audio resampling via tarang-audio.
//!
//! Available only with the `tarang` feature. Provides both fast linear
//! interpolation and high-quality windowed sinc resampling.

use crate::buffer::AudioBuffer;

/// Resample an AudioBuffer to a target sample rate using linear interpolation.
///
/// Fast but lower quality — suitable for previews or non-critical paths.
#[cfg(feature = "tarang")]
pub fn resample(
    buffer: &AudioBuffer,
    source_rate: u32,
    target_rate: u32,
) -> Result<AudioBuffer, Box<dyn std::error::Error>> {
    if source_rate == target_rate {
        return Ok(buffer.clone());
    }

    let tarang_buf = shruti_to_tarang(buffer, source_rate);
    let resampled = tarang_audio::resample(&tarang_buf, target_rate)?;
    Ok(tarang_to_shruti(&resampled))
}

/// Resample an AudioBuffer using high-quality windowed sinc interpolation.
///
/// `window_size` controls the number of sinc lobes (typically 8–64).
/// Higher values give better quality at the cost of CPU.
#[cfg(feature = "tarang")]
pub fn resample_sinc(
    buffer: &AudioBuffer,
    source_rate: u32,
    target_rate: u32,
    window_size: usize,
) -> Result<AudioBuffer, Box<dyn std::error::Error>> {
    if source_rate == target_rate {
        return Ok(buffer.clone());
    }

    let tarang_buf = shruti_to_tarang(buffer, source_rate);
    let resampled = tarang_audio::resample_sinc(&tarang_buf, target_rate, window_size)?;
    Ok(tarang_to_shruti(&resampled))
}

// ── buffer conversion helpers ──────────────────────────────────────────────

#[cfg(feature = "tarang")]
fn shruti_to_tarang(buffer: &AudioBuffer, sample_rate: u32) -> tarang_core::AudioBuffer {
    let interleaved = buffer.as_interleaved();
    let byte_data: Vec<u8> = interleaved.iter().flat_map(|s| s.to_le_bytes()).collect();
    tarang_core::AudioBuffer {
        data: bytes::Bytes::from(byte_data),
        sample_format: tarang_core::SampleFormat::F32,
        channels: buffer.channels(),
        sample_rate,
        num_samples: interleaved.len(),
        timestamp: std::time::Duration::ZERO,
    }
}

#[cfg(feature = "tarang")]
fn tarang_to_shruti(tarang_buf: &tarang_core::AudioBuffer) -> AudioBuffer {
    let float_bytes = &tarang_buf.data;
    let num_floats = float_bytes.len() / 4;
    let mut samples = Vec::with_capacity(num_floats);
    for chunk in float_bytes.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    AudioBuffer::from_interleaved(samples, tarang_buf.channels)
}

#[cfg(test)]
#[cfg(feature = "tarang")]
mod tests {
    use super::*;

    #[test]
    fn resample_same_rate_is_noop() {
        let buf = AudioBuffer::from_interleaved(vec![0.1, -0.1, 0.2, -0.2], 2);
        let result = resample(&buf, 44100, 44100).unwrap();
        assert_eq!(result.frames(), buf.frames());
        assert_eq!(result.channels(), buf.channels());
    }

    #[test]
    fn resample_sinc_same_rate_is_noop() {
        let buf = AudioBuffer::from_interleaved(vec![0.1, -0.1, 0.2, -0.2], 2);
        let result = resample_sinc(&buf, 44100, 44100, 16).unwrap();
        assert_eq!(result.frames(), buf.frames());
    }

    #[test]
    fn resample_changes_length() {
        // 100 frames at 44100 → 48000 should produce ~109 frames
        let samples: Vec<f32> = (0..200).map(|i| (i as f32 * 0.01).sin()).collect();
        let buf = AudioBuffer::from_interleaved(samples, 2);
        let result = resample(&buf, 44100, 48000).unwrap();
        assert!(result.frames() > buf.frames());
        assert_eq!(result.channels(), 2);
    }

    #[test]
    fn resample_sinc_changes_length() {
        let samples: Vec<f32> = (0..200).map(|i| (i as f32 * 0.01).sin()).collect();
        let buf = AudioBuffer::from_interleaved(samples, 2);
        let result = resample_sinc(&buf, 44100, 48000, 16).unwrap();
        assert!(result.frames() > buf.frames());
        assert_eq!(result.channels(), 2);
    }
}
