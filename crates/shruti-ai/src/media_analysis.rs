//! Media analysis integration via tarang-ai.
//!
//! Available only with the `tarang` feature. Provides content classification,
//! audio fingerprinting, AcoustID identification, speaker diarization,
//! and transcription routing for the DAW.

use serde::{Deserialize, Serialize};

// Re-export tarang-ai types for convenience
pub use tarang::ai::{AcoustIdFingerprint, DiarizeConfig, SpeakerSegment};
pub use tarang::ai::{AudioFingerprint, FingerprintConfig};
pub use tarang::ai::{ContentType, MediaAnalysis, TranscriptionResult, TranscriptionSegment};
pub use tarang::ai::{HooshClient, HooshConfig, WhisperModel};

/// Convert interleaved f32 samples into a tarang `AudioBuffer`.
fn to_tarang_buffer(samples: &[f32], sample_rate: u32, channels: u16) -> tarang::core::AudioBuffer {
    let byte_data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    tarang::core::AudioBuffer {
        data: bytes::Bytes::from(byte_data),
        sample_format: tarang::core::SampleFormat::F32,
        channels,
        sample_rate,
        num_frames: samples.len() / channels as usize,
        timestamp: std::time::Duration::ZERO,
    }
}

/// Audio content analysis for a decoded file.
///
/// Wraps tarang-ai's `analyze_media` with a simpler API suited to DAW use
/// cases (audio-only, no video streams).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioAnalysis {
    pub content_type: ContentType,
    pub quality_score: f32,
    pub tags: Vec<String>,
}

/// Analyze an audio file's content type and quality.
///
/// Builds a `MediaInfo` from the given audio parameters and delegates to
/// tarang-ai's classifier. Useful for auto-tagging imported samples.
pub fn analyze_audio(
    sample_rate: u32,
    channels: u16,
    duration_secs: f64,
    codec_name: &str,
) -> AudioAnalysis {
    use std::time::Duration;
    use tarang::core::*;

    let codec = parse_codec(codec_name);

    let info = MediaInfo {
        id: uuid::Uuid::new_v4(),
        format: ContainerFormat::Wav,
        streams: vec![StreamInfo::Audio(AudioStreamInfo {
            codec,
            sample_rate,
            channels,
            sample_format: SampleFormat::F32,
            bitrate: None,
            duration: Some(Duration::from_secs_f64(duration_secs)),
        })],
        duration: Some(Duration::from_secs_f64(duration_secs)),
        file_size: None,
        title: None,
        artist: None,
        album: None,
        metadata: std::collections::HashMap::new(),
    };

    let result = tarang::ai::analyze_media(&info);

    AudioAnalysis {
        content_type: result.content_type,
        quality_score: result.quality_score,
        tags: result.tags,
    }
}

/// Compute an audio fingerprint from interleaved f32 samples.
///
/// The fingerprint can be used for duplicate detection and sample matching.
pub fn fingerprint_audio(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<AudioFingerprint, Box<dyn std::error::Error>> {
    let buf = to_tarang_buffer(samples, sample_rate, channels);
    let config = FingerprintConfig::default();
    let fp = tarang::ai::compute_fingerprint(&buf, &config)?;
    Ok(fp)
}

/// Compute similarity between two fingerprints (0.0 = different, 1.0 = identical).
pub fn fingerprint_similarity(a: &AudioFingerprint, b: &AudioFingerprint) -> f64 {
    tarang::ai::fingerprint_match(a, b)
}

/// Check if two fingerprints match (similarity > 0.8).
pub fn fingerprints_match(a: &AudioFingerprint, b: &AudioFingerprint) -> bool {
    tarang::ai::fingerprint_match(a, b) > 0.8
}

/// Compute an AcoustID-compatible fingerprint from interleaved f32 samples.
///
/// Returns a base64-encoded fingerprint string compatible with the AcoustID
/// music identification database. Useful for sample identification on import.
pub fn compute_acoustid(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<AcoustIdFingerprint, Box<dyn std::error::Error>> {
    let buf = to_tarang_buffer(samples, sample_rate, channels);
    let fp = tarang::ai::compute_acoustid(&buf)?;
    Ok(fp)
}

/// Perform speaker diarization on interleaved f32 audio samples.
///
/// Detects speech segments and assigns speaker labels based on spectral
/// similarity. Useful for podcast/vocal track analysis and auto-splitting
/// multi-speaker recordings into regions.
pub fn diarize_audio(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    config: &DiarizeConfig,
) -> Result<Vec<SpeakerSegment>, Box<dyn std::error::Error>> {
    let buf = to_tarang_buffer(samples, sample_rate, channels);
    let segments = tarang::ai::diarize(&buf, config)?;
    Ok(segments)
}

/// Prepare a transcription request for an audio track.
///
/// Returns a `TranscriptionRequest` that can be sent to the hoosh service
/// for Whisper-based transcription (useful for vocal alignment).
pub fn prepare_transcription(
    sample_rate: u32,
    channels: u16,
    duration_secs: f64,
    codec_name: &str,
    language_hint: Option<String>,
) -> Option<tarang::ai::TranscriptionRequest> {
    use std::time::Duration;
    use tarang::core::*;

    let codec = parse_codec(codec_name);

    let info = MediaInfo {
        id: uuid::Uuid::new_v4(),
        format: ContainerFormat::Wav,
        streams: vec![StreamInfo::Audio(AudioStreamInfo {
            codec,
            sample_rate,
            channels,
            sample_format: SampleFormat::F32,
            bitrate: None,
            duration: Some(Duration::from_secs_f64(duration_secs)),
        })],
        duration: Some(Duration::from_secs_f64(duration_secs)),
        file_size: None,
        title: None,
        artist: None,
        album: None,
        metadata: std::collections::HashMap::new(),
    };

    tarang::ai::prepare_transcription(&info, language_hint)
}

/// Parse a codec name string into a tarang `AudioCodec`.
fn parse_codec(codec_name: &str) -> tarang::core::AudioCodec {
    use tarang::core::AudioCodec;
    match codec_name.to_lowercase().as_str() {
        "wav" | "pcm" => AudioCodec::Pcm,
        "flac" => AudioCodec::Flac,
        "mp3" => AudioCodec::Mp3,
        "aac" | "m4a" => AudioCodec::Aac,
        "ogg" | "vorbis" => AudioCodec::Vorbis,
        "opus" => AudioCodec::Opus,
        "alac" => AudioCodec::Alac,
        _ => AudioCodec::Pcm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_short_audio_is_music() {
        let result = analyze_audio(44100, 2, 240.0, "flac");
        assert_eq!(result.content_type, ContentType::Music);
        assert!(result.tags.contains(&"audio".to_string()));
    }

    #[test]
    fn analyze_long_audio_is_podcast() {
        let result = analyze_audio(44100, 2, 3600.0, "mp3");
        assert_eq!(result.content_type, ContentType::Podcast);
    }

    #[test]
    fn analyze_lossless_has_higher_quality() {
        let lossless = analyze_audio(48000, 2, 120.0, "flac");
        let lossy = analyze_audio(44100, 2, 120.0, "mp3");
        assert!(lossless.quality_score >= lossy.quality_score);
    }

    #[test]
    fn fingerprint_and_match() {
        let samples: Vec<f32> = (0..44100)
            .map(|i| (i as f32 / 44100.0 * 440.0 * std::f32::consts::TAU).sin() * 0.5)
            .collect();
        let fp1 = fingerprint_audio(&samples, 44100, 1).unwrap();
        let fp2 = fingerprint_audio(&samples, 44100, 1).unwrap();
        let similarity = fingerprint_similarity(&fp1, &fp2);
        assert!(
            similarity > 0.5,
            "identical audio should have high similarity, got {similarity}"
        );
    }

    #[test]
    fn acoustid_produces_fingerprint() {
        // Need enough audio for fingerprinting (>= 1 frame of FFT data)
        let samples: Vec<f32> = (0..32000)
            .map(|i| (i as f32 / 16000.0 * 440.0 * std::f32::consts::TAU).sin() * 0.5)
            .collect();
        let result = compute_acoustid(&samples, 16000, 1).unwrap();
        assert!(!result.fingerprint.is_empty());
        assert!(result.duration > 0.0);
    }

    #[test]
    fn acoustid_same_audio_same_fingerprint() {
        let samples: Vec<f32> = (0..32000)
            .map(|i| (i as f32 / 16000.0 * 440.0 * std::f32::consts::TAU).sin() * 0.5)
            .collect();
        let fp1 = compute_acoustid(&samples, 16000, 1).unwrap();
        let fp2 = compute_acoustid(&samples, 16000, 1).unwrap();
        assert_eq!(fp1.fingerprint, fp2.fingerprint);
    }

    #[test]
    fn diarize_silence_returns_empty() {
        let samples = vec![0.0f32; 32000]; // 2 seconds at 16kHz
        let config = DiarizeConfig::default();
        let segments = diarize_audio(&samples, 16000, 1, &config).unwrap();
        assert!(segments.is_empty(), "silence should produce no segments");
    }

    #[test]
    fn diarize_tone_produces_segments() {
        let samples: Vec<f32> = (0..32000)
            .map(|i| (i as f32 / 16000.0 * 440.0 * std::f32::consts::TAU).sin() * 0.5)
            .collect();
        let config = DiarizeConfig::default();
        let segments = diarize_audio(&samples, 16000, 1, &config).unwrap();
        assert!(!segments.is_empty(), "a tone should produce segments");
        // All segments should be one speaker
        let ids: Vec<u32> = segments.iter().map(|s| s.speaker_id).collect();
        assert!(ids.iter().all(|&id| id == ids[0]));
    }

    #[test]
    fn prepare_transcription_returns_request() {
        let req = prepare_transcription(44100, 2, 120.0, "wav", Some("en".to_string()));
        assert!(req.is_some());
        let req = req.unwrap();
        assert_eq!(req.sample_rate, 44100);
        assert_eq!(req.channels, 2);
        assert_eq!(req.language_hint, Some("en".to_string()));
    }
}
