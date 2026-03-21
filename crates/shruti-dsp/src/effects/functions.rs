//! Stateless DSP functions — noise gate, hard limiter, normalize.

use crate::buffer::AudioBuffer;

/// Silence samples below the given linear threshold.
pub fn noise_gate(buffer: &mut AudioBuffer, threshold: f32) {
    if let Ok(mut dbuf) = dhvani::buffer::AudioBuffer::from_interleaved(
        buffer.as_interleaved().to_vec(),
        buffer.channels() as u32,
        48000, // sample rate not used by noise_gate
    ) {
        dhvani::dsp::noise_gate(&mut dbuf, threshold);
        buffer.as_interleaved_mut().copy_from_slice(&dbuf.samples);
    }
}

/// Clamp samples at the given ceiling (absolute value).
pub fn hard_limiter(buffer: &mut AudioBuffer, ceiling: f32) {
    if let Ok(mut dbuf) = dhvani::buffer::AudioBuffer::from_interleaved(
        buffer.as_interleaved().to_vec(),
        buffer.channels() as u32,
        48000,
    ) {
        dhvani::dsp::hard_limiter(&mut dbuf, ceiling);
        buffer.as_interleaved_mut().copy_from_slice(&dbuf.samples);
    }
}

/// Normalize buffer so the peak reaches the target level (linear).
pub fn normalize(buffer: &mut AudioBuffer, target_peak: f32) {
    if let Ok(mut dbuf) = dhvani::buffer::AudioBuffer::from_interleaved(
        buffer.as_interleaved().to_vec(),
        buffer.channels() as u32,
        48000,
    ) {
        dhvani::dsp::normalize(&mut dbuf, target_peak);
        buffer.as_interleaved_mut().copy_from_slice(&dbuf.samples);
    }
}

/// Convert amplitude to decibels.
pub fn amplitude_to_db(amplitude: f32) -> f32 {
    dhvani::dsp::amplitude_to_db(amplitude)
}

/// Convert decibels to amplitude.
pub fn db_to_amplitude(db: f32) -> f32 {
    dhvani::dsp::db_to_amplitude(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_gate_silences_quiet() {
        let mut buf = AudioBuffer::from_interleaved(vec![0.01, -0.01, 0.5, -0.5], 1);
        noise_gate(&mut buf, 0.1);
        assert_eq!(buf.get(0, 0), 0.0);
        assert_eq!(buf.get(1, 0), 0.0);
        assert!(buf.get(2, 0).abs() > 0.1);
    }

    #[test]
    fn hard_limiter_clamps() {
        let mut buf = AudioBuffer::from_interleaved(vec![1.5, -1.5, 0.3, -0.3], 1);
        hard_limiter(&mut buf, 0.5);
        for s in buf.as_interleaved() {
            assert!(s.abs() <= 0.5 + f32::EPSILON);
        }
    }

    #[test]
    fn normalize_to_peak() {
        let mut buf = AudioBuffer::from_interleaved(vec![0.25, -0.25, 0.5, -0.5], 1);
        normalize(&mut buf, 1.0);
        let peak = buf
            .as_interleaved()
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        assert!((peak - 1.0).abs() < 0.01);
    }

    #[test]
    fn db_conversion_roundtrip() {
        let amp = 0.5f32;
        let db = amplitude_to_db(amp);
        let back = db_to_amplitude(db);
        assert!((back - amp).abs() < 0.001);
    }

    #[test]
    fn output_finite() {
        let samples: Vec<f32> = (0..256).map(|i| (i as f32 * 0.02).sin()).collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 1);
        noise_gate(&mut buf, 0.01);
        hard_limiter(&mut buf, 0.8);
        normalize(&mut buf, 0.95);
        assert!(buf.as_interleaved().iter().all(|s| s.is_finite()));
    }
}
