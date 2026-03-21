//! Dynamics analysis — backed by dhvani.

pub use dhvani::analysis::DynamicsAnalysis;

use crate::AudioBuffer;

/// Perform per-channel dynamics analysis on an AudioBuffer.
///
/// Returns peak, RMS, true peak, crest factor, LUFS, and dynamic range
/// per channel. The `sample_rate` parameter is used for buffer conversion.
pub fn analyze_dynamics(buffer: &AudioBuffer, sample_rate: u32) -> DynamicsAnalysis {
    if let Ok(dbuf) = dhvani::buffer::AudioBuffer::from_interleaved(
        buffer.as_interleaved().to_vec(),
        buffer.channels() as u32,
        sample_rate,
    ) {
        dhvani::analysis::analyze_dynamics(&dbuf)
    } else {
        // Fallback for invalid buffers
        let ch = buffer.channels() as usize;
        DynamicsAnalysis {
            peak: vec![0.0; ch],
            peak_db: vec![f32::NEG_INFINITY; ch],
            true_peak: vec![0.0; ch],
            true_peak_db: vec![f32::NEG_INFINITY; ch],
            rms: vec![0.0; ch],
            rms_db: vec![f32::NEG_INFINITY; ch],
            crest_factor_db: vec![0.0; ch],
            lufs: f32::NEG_INFINITY,
            dynamic_range_db: 0.0,
            frame_count: 0,
            channel_count: buffer.channels() as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AudioBuffer;

    #[test]
    fn silence_analysis() {
        let buf = AudioBuffer::new(2, 1024);
        let result = analyze_dynamics(&buf, 48000);
        assert_eq!(result.channel_count, 2);
        assert_eq!(result.frame_count, 1024);
        for &p in &result.peak {
            assert_eq!(p, 0.0);
        }
        for &r in &result.rms {
            assert_eq!(r, 0.0);
        }
    }

    #[test]
    fn unity_sine_peak_and_rms() {
        let frames = 48000u32;
        let mut buf = AudioBuffer::new(1, frames);
        for i in 0..frames {
            let s = (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48000.0).sin() as f32;
            buf.set(i, 0, s);
        }
        let result = analyze_dynamics(&buf, 48000);
        assert!((result.peak[0] - 1.0).abs() < 0.01);
        assert!(
            (result.rms[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.01,
            "RMS was {}, expected ~0.707",
            result.rms[0]
        );
        assert!(
            (result.crest_factor_db[0] - 3.01).abs() < 0.5,
            "Crest factor was {} dB, expected ~3.01 dB",
            result.crest_factor_db[0]
        );
    }

    #[test]
    fn dc_signal_has_zero_crest_factor() {
        let frames = 1024u32;
        let mut buf = AudioBuffer::new(1, frames);
        for i in 0..frames {
            buf.set(i, 0, 0.5);
        }
        let result = analyze_dynamics(&buf, 48000);
        assert!(
            (result.crest_factor_db[0]).abs() < 0.01,
            "Crest factor was {} dB, expected ~0 dB",
            result.crest_factor_db[0]
        );
    }

    #[test]
    fn peak_db_correct() {
        let mut buf = AudioBuffer::new(1, 100);
        buf.set(50, 0, 0.5);
        let result = analyze_dynamics(&buf, 48000);
        assert!((result.peak_db[0] - (-6.02)).abs() < 0.1);
    }

    #[test]
    fn true_peak_catches_intersample() {
        let mut buf = AudioBuffer::new(1, 2);
        buf.set(0, 0, 0.9);
        buf.set(1, 0, -0.9);
        let result = analyze_dynamics(&buf, 48000);
        assert!(result.true_peak[0] >= result.peak[0]);
    }

    #[test]
    fn dynamic_range_positive_for_signal() {
        let frames = 4096u32;
        let mut buf = AudioBuffer::new(1, frames);
        for i in 0..frames {
            let s = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48000.0).sin() as f32;
            buf.set(i, 0, s);
        }
        let result = analyze_dynamics(&buf, 48000);
        assert!(result.dynamic_range_db > 0.0);
    }

    #[test]
    fn test_empty_buffer() {
        let buf = AudioBuffer::new(1, 0);
        let result = analyze_dynamics(&buf, 48000);
        assert_eq!(result.frame_count, 0);
        assert_eq!(result.channel_count, 1);
        assert_eq!(result.peak[0], 0.0);
        assert_eq!(result.rms[0], 0.0);
    }

    #[test]
    fn test_single_frame() {
        let mut buf = AudioBuffer::new(1, 1);
        buf.set(0, 0, 0.75);
        let result = analyze_dynamics(&buf, 48000);
        assert!((result.peak[0] - 0.75).abs() < 0.001);
        assert!((result.rms[0] - 0.75).abs() < 0.001);
        assert!((result.true_peak[0] - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_stereo_independent_channels() {
        let mut buf = AudioBuffer::new(2, 1024);
        for i in 0..1024 {
            buf.set(i, 0, 0.8);
            buf.set(i, 1, 0.1);
        }
        let result = analyze_dynamics(&buf, 48000);
        assert!((result.peak[0] - 0.8).abs() < 0.001);
        assert!((result.peak[1] - 0.1).abs() < 0.001);
        assert!(result.peak_db[0] > result.peak_db[1]);
    }

    #[test]
    fn test_lufs_finite_for_signal() {
        let frames = 48000u32;
        let mut buf = AudioBuffer::new(1, frames);
        for i in 0..frames {
            let s = (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48000.0).sin() as f32;
            buf.set(i, 0, s * 0.5);
        }
        let result = analyze_dynamics(&buf, 48000);
        assert!(result.lufs.is_finite(), "LUFS should be finite for a real signal");
        assert!(result.lufs < 0.0, "LUFS should be negative");
    }
}
