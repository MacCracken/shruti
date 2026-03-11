use std::path::Path;

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::buffer::AudioBuffer;
use crate::format::AudioFormat;

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
}
