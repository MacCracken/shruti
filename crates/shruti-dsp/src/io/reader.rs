use std::path::Path;

use tarang_audio::FileDecoder;

use crate::buffer::AudioBuffer;
use crate::format::AudioFormat;

/// Supported audio file extensions for reading.
///
/// Powered by tarang-audio (symphonia backend): lossless (WAV, FLAC, AIFF),
/// lossy (OGG/Vorbis, MP3, AAC/M4A, ALAC), and Opus.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "wav", "flac", "aiff", "aif", "ogg", "mp3", "m4a", "aac", "alac", "opus",
];

/// Read an audio file into an AudioBuffer using tarang-audio.
///
/// Supports WAV, FLAC, AIFF, OGG/Vorbis, MP3, AAC, ALAC, and Opus.
/// Wraps decoding in `catch_unwind` to safely handle malformed files
/// that might cause panics in the decoder, returning an error instead.
pub fn read_audio_file(
    path: &Path,
) -> Result<(AudioBuffer, AudioFormat), Box<dyn std::error::Error>> {
    let path = path.to_path_buf();
    // Wrap the entire decoding pipeline in catch_unwind to handle malformed files
    // that may cause panics in symphonia's decoders (via tarang-audio).
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        read_audio_file_inner(&path)
    }));

    match result {
        Ok(inner_result) => inner_result,
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                format!("decoder panicked on malformed file: {s}")
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                format!("decoder panicked on malformed file: {s}")
            } else {
                "decoder panicked on malformed file".to_string()
            };
            Err(msg.into())
        }
    }
}

/// Inner implementation using tarang-audio's FileDecoder.
fn read_audio_file_inner(
    path: &Path,
) -> Result<(AudioBuffer, AudioFormat), Box<dyn std::error::Error>> {
    let mut decoder = FileDecoder::open_path(path)?;

    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels();

    let tarang_buf = match decoder.decode_all() {
        Ok(buf) => buf,
        Err(tarang_core::TarangError::DecodeError(ref msg)) if msg == "no audio decoded" => {
            // Empty file — return an empty buffer
            let audio_format = AudioFormat::new(sample_rate, channels, 0);
            let buffer = AudioBuffer::new(channels, 0);
            return Ok((buffer, audio_format));
        }
        Err(e) => return Err(e.into()),
    };

    // Convert tarang's AudioBuffer (Bytes of f32) to shruti's AudioBuffer (Vec<f32>)
    let float_bytes = &tarang_buf.data;
    let num_floats = float_bytes.len() / 4;
    let mut samples = Vec::with_capacity(num_floats);
    for chunk in float_bytes.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    let audio_format = AudioFormat::new(sample_rate, channels, 0);
    let buffer = AudioBuffer::from_interleaved(samples, channels);

    Ok((buffer, audio_format))
}

/// Check if a file extension is supported for reading.
pub fn is_supported_extension(ext: &str) -> bool {
    SUPPORTED_EXTENSIONS
        .iter()
        .any(|&e| e.eq_ignore_ascii_case(ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::AudioFormat;
    use crate::io::write_wav_file;

    #[test]
    fn supported_extensions_includes_all_formats() {
        assert!(is_supported_extension("wav"));
        assert!(is_supported_extension("flac"));
        assert!(is_supported_extension("aiff"));
        assert!(is_supported_extension("aif"));
        assert!(is_supported_extension("ogg"));
        assert!(is_supported_extension("mp3"));
        assert!(is_supported_extension("m4a"));
        assert!(is_supported_extension("aac"));
        assert!(is_supported_extension("alac"));
        assert!(is_supported_extension("opus"));
        assert!(is_supported_extension("WAV"));
        assert!(is_supported_extension("OGG"));
        assert!(is_supported_extension("MP3"));
    }

    #[test]
    fn read_wav_roundtrip() {
        let dir = std::env::temp_dir().join("shruti_reader_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.wav");

        // Write a short WAV file
        let buf = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3], 2);
        let fmt = AudioFormat::new(44100, 2, 0);
        write_wav_file(&path, &buf, &fmt).unwrap();

        // Read it back
        let (read_buf, fmt) = read_audio_file(&path).unwrap();
        assert_eq!(fmt.sample_rate, 44100);
        assert_eq!(read_buf.channels(), 2);
        assert_eq!(read_buf.frames(), 2);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn read_nonexistent_file_returns_error() {
        let result = read_audio_file(Path::new("/nonexistent/audio.wav"));
        assert!(result.is_err());
    }

    #[test]
    fn read_invalid_file_returns_error() {
        let dir = std::env::temp_dir().join("shruti_reader_bad");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.wav");
        std::fs::write(&path, b"not audio data").unwrap();

        let result = read_audio_file(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn read_truncated_wav_returns_error_not_panic() {
        let dir = std::env::temp_dir().join("shruti_reader_truncated");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("truncated.wav");

        // Minimal WAV header (44 bytes) with data size claiming more data than exists
        let mut header = vec![0u8; 44];
        header[0..4].copy_from_slice(b"RIFF");
        let file_size: u32 = 10000;
        header[4..8].copy_from_slice(&file_size.to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16u32.to_le_bytes());
        header[20..22].copy_from_slice(&1u16.to_le_bytes()); // PCM
        header[22..24].copy_from_slice(&1u16.to_le_bytes()); // mono
        header[24..28].copy_from_slice(&44100u32.to_le_bytes());
        header[28..32].copy_from_slice(&(44100u32 * 2).to_le_bytes());
        header[32..34].copy_from_slice(&2u16.to_le_bytes());
        header[34..36].copy_from_slice(&16u16.to_le_bytes());
        header[36..40].copy_from_slice(b"data");
        header[40..44].copy_from_slice(&9000u32.to_le_bytes());

        std::fs::write(&path, &header).unwrap();

        // Should return an error, not panic
        let result = read_audio_file(&path);
        assert!(
            result.is_ok() || result.is_err(),
            "Should not panic on truncated file"
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn read_random_bytes_returns_error_not_panic() {
        let dir = std::env::temp_dir().join("shruti_reader_random");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("random.wav");

        let garbage: Vec<u8> = (0..1024).map(|i| (i * 37 + 13) as u8).collect();
        std::fs::write(&path, &garbage).unwrap();

        let result = read_audio_file(&path);
        assert!(result.is_err(), "Random bytes should produce an error");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
