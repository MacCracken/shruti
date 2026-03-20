pub mod reader;
pub mod writer;

#[cfg(feature = "tarang")]
pub mod loudness;
#[cfg(feature = "tarang")]
pub mod mix;
#[cfg(feature = "tarang")]
pub mod resample;

pub use reader::{SUPPORTED_EXTENSIONS, read_audio_file};
pub use writer::{
    BitDepth, ExportConfig, ExportFormat, is_native_export, write_audio_file, write_wav_file,
};

/// Convert a shruti AudioBuffer to a tarang AudioBuffer.
#[cfg(feature = "tarang")]
pub fn shruti_to_tarang_buf(
    buffer: &crate::buffer::AudioBuffer,
    format: &crate::format::AudioFormat,
) -> tarang::core::AudioBuffer {
    let interleaved = buffer.as_interleaved();
    let byte_data: Vec<u8> = interleaved.iter().flat_map(|s| s.to_le_bytes()).collect();
    debug_assert!(buffer.channels() > 0, "AudioBuffer has 0 channels");
    tarang::core::AudioBuffer {
        data: bytes::Bytes::from(byte_data),
        sample_format: tarang::core::SampleFormat::F32,
        channels: buffer.channels().max(1),
        sample_rate: format.sample_rate,
        num_frames: buffer.frames() as usize,
        timestamp: std::time::Duration::ZERO,
    }
}

/// Convert a tarang AudioBuffer back to a shruti AudioBuffer.
#[cfg(feature = "tarang")]
pub fn tarang_buf_to_shruti(tarang_buf: &tarang::core::AudioBuffer) -> crate::buffer::AudioBuffer {
    let float_bytes = &tarang_buf.data;
    let num_floats = float_bytes.len() / 4;
    let mut samples = Vec::with_capacity(num_floats);
    for chunk in float_bytes.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    crate::buffer::AudioBuffer::from_interleaved(samples, tarang_buf.channels)
}

#[cfg(feature = "tarang")]
pub use loudness::{LoudnessMetrics, apply_gain, measure_loudness, normalize_loudness};
#[cfg(feature = "tarang")]
pub use mix::{ChannelLayout, mix_channels};
#[cfg(feature = "tarang")]
pub use reader::{ContainerInfo, StreamingError, StreamingReader, probe_container};
#[cfg(feature = "tarang")]
pub use resample::{resample, resample_sinc};
