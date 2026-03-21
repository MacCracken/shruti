//! EBU R128 loudness measurement — backed by dhvani.

pub use dhvani::analysis::R128Loudness;

use crate::AudioBuffer;

/// Measure EBU R128 integrated loudness.
///
/// Returns integrated LUFS, loudness range, short-term, and momentary values.
pub fn measure_r128(buf: &AudioBuffer, sample_rate: u32) -> Option<R128Loudness> {
    let dbuf = dhvani::buffer::AudioBuffer::from_interleaved(
        buf.as_interleaved().to_vec(),
        buf.channels() as u32,
        sample_rate,
    )
    .ok()?;
    Some(dhvani::analysis::measure_r128(&dbuf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_very_low_lufs() {
        let buf = AudioBuffer::new(1, 48000);
        let r128 = measure_r128(&buf, 48000).unwrap();
        assert!(r128.integrated_lufs < -60.0, "silence should be very quiet");
    }

    #[test]
    fn signal_returns_finite() {
        let sr = 48000u32;
        let mut buf = AudioBuffer::new(1, sr);
        for i in 0..sr {
            let s = (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / sr as f64).sin() as f32 * 0.5;
            buf.set(i, 0, s);
        }
        let r128 = measure_r128(&buf, sr).unwrap();
        assert!(r128.integrated_lufs.is_finite());
        assert!(r128.integrated_lufs < 0.0);
        assert!(r128.range_lu >= 0.0);
    }
}
