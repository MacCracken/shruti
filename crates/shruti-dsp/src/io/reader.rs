use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::buffer::AudioBuffer;
use crate::format::AudioFormat;

/// Supported audio file extensions for reading.
pub const SUPPORTED_EXTENSIONS: &[&str] = &["wav", "flac", "aiff", "aif", "ogg"];

/// Read an audio file (WAV, FLAC, AIFF, OGG/Vorbis) into an AudioBuffer.
pub fn read_audio_file(
    path: &Path,
) -> Result<(AudioBuffer, AudioFormat), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format.tracks().first().ok_or("no audio tracks found")?;

    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(2);
    let sample_rate = track.codec_params.sample_rate.unwrap_or(48000);

    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let track_id = track.id;
    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet)?;
        let spec = *decoded.spec();
        let duration = decoded.capacity();

        let mut sample_buf = SampleBuffer::<f32>::new(duration as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        all_samples.extend_from_slice(sample_buf.samples());
    }

    let audio_format = AudioFormat::new(sample_rate, channels, 0);
    let buffer = AudioBuffer::from_interleaved(all_samples, channels);

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
        assert!(is_supported_extension("WAV"));
        assert!(is_supported_extension("OGG"));
        assert!(!is_supported_extension("mp3"));
        assert!(!is_supported_extension("m4a"));
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
}
