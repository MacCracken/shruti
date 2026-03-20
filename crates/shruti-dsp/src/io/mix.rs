//! Channel mixing via tarang-audio.
//!
//! Available only with the `tarang` feature. Provides stereo/mono conversion
//! and multichannel downmixing (including 5.1 → stereo via ITU-R BS.775).

use crate::buffer::AudioBuffer;

/// Target channel layout for mixing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLayout {
    Mono,
    Stereo,
}

/// Mix an AudioBuffer to the target channel layout.
///
/// Supports: stereo→mono, mono→stereo, 5.1→stereo, 5.1→mono,
/// and generic N-channel→mono/stereo fallbacks.
#[cfg(feature = "tarang")]
pub fn mix_channels(
    buffer: &AudioBuffer,
    source_rate: u32,
    target: ChannelLayout,
) -> Result<AudioBuffer, Box<dyn std::error::Error>> {
    let target_layout = match target {
        ChannelLayout::Mono => tarang::audio::ChannelLayout::Mono,
        ChannelLayout::Stereo => tarang::audio::ChannelLayout::Stereo,
    };

    let interleaved = buffer.as_interleaved();
    let byte_data: Vec<u8> = interleaved.iter().flat_map(|s| s.to_le_bytes()).collect();
    let tarang_buf = tarang::core::AudioBuffer {
        data: bytes::Bytes::from(byte_data),
        sample_format: tarang::core::SampleFormat::F32,
        channels: buffer.channels(),
        sample_rate: source_rate,
        num_frames: buffer.frames() as usize,
        timestamp: std::time::Duration::ZERO,
    };

    let mixed = tarang::audio::mix_channels(&tarang_buf, target_layout)?;

    let float_bytes = &mixed.data;
    let num_floats = float_bytes.len() / 4;
    let mut samples = Vec::with_capacity(num_floats);
    for chunk in float_bytes.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    Ok(AudioBuffer::from_interleaved(samples, mixed.channels))
}

#[cfg(test)]
#[cfg(feature = "tarang")]
mod tests {
    use super::*;

    #[test]
    fn stereo_to_mono() {
        let buf = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3], 2);
        let result = mix_channels(&buf, 44100, ChannelLayout::Mono).unwrap();
        assert_eq!(result.channels(), 1);
        assert_eq!(result.frames(), 2);
    }

    #[test]
    fn mono_to_stereo() {
        let buf = AudioBuffer::from_interleaved(vec![0.5, 0.3, 0.1], 1);
        let result = mix_channels(&buf, 44100, ChannelLayout::Stereo).unwrap();
        assert_eq!(result.channels(), 2);
        assert_eq!(result.frames(), 3);
    }

    #[test]
    fn mono_to_mono_is_identity() {
        let buf = AudioBuffer::from_interleaved(vec![0.5, 0.3], 1);
        let result = mix_channels(&buf, 44100, ChannelLayout::Mono).unwrap();
        assert_eq!(result.channels(), 1);
        assert_eq!(result.frames(), 2);
        let diff = (result.as_interleaved()[0] - 0.5).abs();
        assert!(diff < 1e-6);
    }
}
