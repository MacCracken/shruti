//! Shared test utilities for Shruti crates.
//!
//! Provides common helpers used across unit and integration tests:
//! sine generation, RMS measurement, silence detection, and more.

use std::f32::consts::PI;

use shruti_dsp::AudioBuffer;

/// Generate a mono sine wave at the given frequency.
pub fn generate_sine(freq: f32, sample_rate: f32, frames: usize, amplitude: f32) -> Vec<f32> {
    (0..frames)
        .map(|i| (2.0 * PI * freq * i as f32 / sample_rate).sin() * amplitude)
        .collect()
}

/// Compute RMS of a single channel in a buffer.
pub fn rms_of_buffer(buf: &AudioBuffer, channel: u16, frames: usize) -> f32 {
    let sum: f32 = (0..frames)
        .map(|i| buf.get(i as u32, channel).powi(2))
        .sum();
    (sum / frames as f32).sqrt()
}

/// Check that a buffer has non-silent content (any sample above threshold).
pub fn has_audio(buf: &AudioBuffer, threshold: f32) -> bool {
    for frame in 0..buf.frames() {
        for ch in 0..buf.channels() {
            if buf.get(frame, ch).abs() > threshold {
                return true;
            }
        }
    }
    false
}

/// Check whether a buffer is entirely silent (all samples below threshold).
pub fn is_silent(buf: &AudioBuffer, threshold: f32) -> bool {
    !has_audio(buf, threshold)
}

/// Count zero crossings on a single channel.
pub fn count_zero_crossings(buf: &AudioBuffer, channel: u16, frames: u32) -> usize {
    let mut count = 0;
    for i in 1..frames {
        let prev = buf.get(i - 1, channel);
        let curr = buf.get(i, channel);
        if (prev >= 0.0 && curr < 0.0) || (prev < 0.0 && curr >= 0.0) {
            count += 1;
        }
    }
    count
}

/// Fill an AudioBuffer with a mono sine on all channels.
pub fn fill_sine(buf: &mut AudioBuffer, freq: f32, sample_rate: f32, amplitude: f32) {
    for frame in 0..buf.frames() {
        let sample = (2.0 * PI * freq * frame as f32 / sample_rate).sin() * amplitude;
        for ch in 0..buf.channels() {
            buf.set(frame, ch, sample);
        }
    }
}

/// Compute peak amplitude across all channels.
pub fn peak_amplitude(buf: &AudioBuffer) -> f32 {
    let mut peak: f32 = 0.0;
    for frame in 0..buf.frames() {
        for ch in 0..buf.channels() {
            peak = peak.max(buf.get(frame, ch).abs());
        }
    }
    peak
}

/// Build NoteEvents from a MidiClip at a specific frame position.
pub fn collect_note_ons(
    clip: &shruti_session::midi::MidiClip,
    frame: u64,
) -> Vec<shruti_session::midi::NoteEvent> {
    clip.note_ons_at(shruti_session::FramePos(frame))
        .into_iter()
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sine_length() {
        let sine = generate_sine(440.0, 44100.0, 1000, 1.0);
        assert_eq!(sine.len(), 1000);
    }

    #[test]
    fn test_generate_sine_amplitude() {
        let sine = generate_sine(440.0, 44100.0, 44100, 0.5);
        let max = sine.iter().cloned().fold(0.0f32, f32::max);
        assert!(max <= 0.501); // allow small float error
        assert!(max >= 0.499);
    }

    #[test]
    fn test_generate_sine_starts_at_zero() {
        let sine = generate_sine(440.0, 44100.0, 100, 1.0);
        assert!(sine[0].abs() < 1e-6);
    }

    #[test]
    fn test_rms_of_silence() {
        let buf = AudioBuffer::new(2, 256);
        let rms = rms_of_buffer(&buf, 0, 256);
        assert!(rms < 1e-7);
    }

    #[test]
    fn test_rms_of_dc() {
        let samples = vec![0.5; 200];
        let buf = AudioBuffer::from_interleaved(samples, 1);
        let rms = rms_of_buffer(&buf, 0, 200);
        assert!((rms - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_has_audio_silent() {
        let buf = AudioBuffer::new(2, 128);
        assert!(!has_audio(&buf, 0.001));
    }

    #[test]
    fn test_has_audio_nonsilent() {
        let mut buf = AudioBuffer::new(1, 128);
        buf.set(64, 0, 0.5);
        assert!(has_audio(&buf, 0.001));
    }

    #[test]
    fn test_is_silent() {
        let buf = AudioBuffer::new(2, 128);
        assert!(is_silent(&buf, 0.001));
    }

    #[test]
    fn test_count_zero_crossings_sine() {
        let sine = generate_sine(100.0, 44100.0, 44100, 1.0);
        let buf = AudioBuffer::from_interleaved(sine, 1);
        let crossings = count_zero_crossings(&buf, 0, 44100);
        // 100 Hz sine should cross zero ~200 times per second
        assert!((crossings as i32 - 200).abs() < 5);
    }

    #[test]
    fn test_fill_sine() {
        let mut buf = AudioBuffer::new(2, 256);
        fill_sine(&mut buf, 440.0, 44100.0, 0.8);
        assert!(has_audio(&buf, 0.1));
        assert!(peak_amplitude(&buf) <= 0.801);
    }

    #[test]
    fn test_peak_amplitude_silence() {
        let buf = AudioBuffer::new(1, 128);
        assert_eq!(peak_amplitude(&buf), 0.0);
    }

    #[test]
    fn test_peak_amplitude_known_value() {
        let mut buf = AudioBuffer::new(1, 128);
        buf.set(42, 0, -0.9);
        assert!((peak_amplitude(&buf) - 0.9).abs() < 1e-6);
    }
}
