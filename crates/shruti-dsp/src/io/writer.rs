use std::path::Path;

use hound::{SampleFormat, WavSpec, WavWriter};
use serde::{Deserialize, Serialize};

use crate::buffer::AudioBuffer;
use crate::format::AudioFormat;

/// Supported export audio formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Wav,
    Flac,
}

/// Bit depth options for export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitDepth {
    Int16,
    Int24,
    Float32,
}

/// Export configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub bit_depth: BitDepth,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Wav,
            bit_depth: BitDepth::Float32,
            sample_rate: 48000,
            channels: 2,
        }
    }
}

/// Write an AudioBuffer to a WAV file.
pub fn write_wav_file(
    path: &Path,
    buffer: &AudioBuffer,
    format: &AudioFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec = WavSpec {
        channels: buffer.channels(),
        sample_rate: format.sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = WavWriter::create(path, spec)?;

    for &sample in buffer.as_interleaved() {
        writer.write_sample(sample)?;
    }

    writer.finalize()?;
    Ok(())
}

/// Write an AudioBuffer to an audio file using the given export configuration.
///
/// Currently supports WAV format with Int16, Int24, and Float32 bit depths.
/// FLAC export is planned for a future release and currently falls back to WAV.
pub fn write_audio_file(
    path: &Path,
    buffer: &AudioBuffer,
    config: &ExportConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    match config.format {
        ExportFormat::Wav | ExportFormat::Flac => {
            // FLAC writing is not supported by hound; fall back to WAV encoding.
            // A future release will add native FLAC export.
            write_wav_with_depth(path, buffer, config)
        }
    }
}

fn write_wav_with_depth(
    path: &Path,
    buffer: &AudioBuffer,
    config: &ExportConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let (bits_per_sample, sample_format) = match config.bit_depth {
        BitDepth::Int16 => (16, SampleFormat::Int),
        BitDepth::Int24 => (24, SampleFormat::Int),
        BitDepth::Float32 => (32, SampleFormat::Float),
    };

    let spec = WavSpec {
        channels: config.channels,
        sample_rate: config.sample_rate,
        bits_per_sample,
        sample_format,
    };

    let mut writer = WavWriter::create(path, spec)?;

    match config.bit_depth {
        BitDepth::Int16 => {
            for &sample in buffer.as_interleaved() {
                let clamped = sample.clamp(-1.0, 1.0);
                let int_sample = (clamped * 32767.0) as i16;
                writer.write_sample(int_sample)?;
            }
        }
        BitDepth::Int24 => {
            for &sample in buffer.as_interleaved() {
                let clamped = sample.clamp(-1.0, 1.0);
                let int_sample = (clamped * 8_388_607.0) as i32;
                writer.write_sample(int_sample)?;
            }
        }
        BitDepth::Float32 => {
            for &sample in buffer.as_interleaved() {
                writer.write_sample(sample)?;
            }
        }
    }

    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::reader::read_audio_file;

    #[test]
    fn test_wav_roundtrip() {
        let original =
            AudioBuffer::from_interleaved(vec![0.1, -0.2, 0.3, -0.4, 0.5, -0.6, 0.7, -0.8], 2);
        let format = AudioFormat::new(48000, 2, 0);

        let tmp = std::env::temp_dir().join("shruti_test_roundtrip.wav");
        write_wav_file(&tmp, &original, &format).unwrap();

        let (loaded, loaded_format) = read_audio_file(&tmp).unwrap();
        assert_eq!(loaded_format.sample_rate, 48000);
        assert_eq!(loaded_format.channels, 2);
        assert_eq!(loaded.frames(), original.frames());

        for i in 0..original.sample_count() {
            let diff = (original.as_interleaved()[i] - loaded.as_interleaved()[i]).abs();
            assert!(
                diff < 1e-6,
                "sample {} differs: {} vs {}",
                i,
                original.as_interleaved()[i],
                loaded.as_interleaved()[i]
            );
        }

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_wav_16bit_roundtrip() {
        let original =
            AudioBuffer::from_interleaved(vec![0.1, -0.2, 0.3, -0.4, 0.5, -0.6, 0.7, -0.8], 2);

        let config = ExportConfig {
            format: ExportFormat::Wav,
            bit_depth: BitDepth::Int16,
            sample_rate: 48000,
            channels: 2,
        };

        let tmp = std::env::temp_dir().join("shruti_test_16bit_roundtrip.wav");
        write_audio_file(&tmp, &original, &config).unwrap();

        let (loaded, loaded_format) = read_audio_file(&tmp).unwrap();
        assert_eq!(loaded_format.sample_rate, 48000);
        assert_eq!(loaded_format.channels, 2);
        assert_eq!(loaded.frames(), original.frames());

        // 16-bit quantization error: 1 / 32768 ≈ 3.05e-5, plus floating-point rounding
        let tolerance = 1.0 / 32768.0 + 1e-4;
        for i in 0..original.sample_count() {
            let diff = (original.as_interleaved()[i] - loaded.as_interleaved()[i]).abs();
            assert!(
                diff < tolerance,
                "sample {} differs beyond 16-bit tolerance: {} vs {} (diff {})",
                i,
                original.as_interleaved()[i],
                loaded.as_interleaved()[i],
                diff,
            );
        }

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_wav_24bit_roundtrip() {
        let original =
            AudioBuffer::from_interleaved(vec![0.1, -0.2, 0.3, -0.4, 0.5, -0.6, 0.7, -0.8], 2);

        let config = ExportConfig {
            format: ExportFormat::Wav,
            bit_depth: BitDepth::Int24,
            sample_rate: 48000,
            channels: 2,
        };

        let tmp = std::env::temp_dir().join("shruti_test_24bit_roundtrip.wav");
        write_audio_file(&tmp, &original, &config).unwrap();

        let (loaded, loaded_format) = read_audio_file(&tmp).unwrap();
        assert_eq!(loaded_format.sample_rate, 48000);
        assert_eq!(loaded_format.channels, 2);
        assert_eq!(loaded.frames(), original.frames());

        // 24-bit quantization error: 1 / 8388607 ≈ 1.19e-7
        let tolerance = 1.0 / 8_388_607.0 + 1e-6;
        for i in 0..original.sample_count() {
            let diff = (original.as_interleaved()[i] - loaded.as_interleaved()[i]).abs();
            assert!(
                diff < tolerance,
                "sample {} differs beyond 24-bit tolerance: {} vs {} (diff {})",
                i,
                original.as_interleaved()[i],
                loaded.as_interleaved()[i],
                diff,
            );
        }

        std::fs::remove_file(&tmp).ok();
    }
}
