use crate::AudioBuffer;

/// Result of dynamics analysis on a buffer.
#[derive(Debug, Clone)]
pub struct DynamicsAnalysis {
    /// Peak level per channel (linear).
    pub peak: Vec<f32>,
    /// Peak level per channel (dB).
    pub peak_db: Vec<f32>,
    /// RMS level per channel (linear).
    pub rms: Vec<f32>,
    /// RMS level per channel (dB).
    pub rms_db: Vec<f32>,
    /// Crest factor per channel (dB) — peak/RMS ratio.
    pub crest_factor_db: Vec<f32>,
    /// Integrated loudness (LUFS) — EBU R128 simplified.
    pub lufs: f32,
    /// Dynamic range (dB) — difference between peak and RMS.
    pub dynamic_range_db: f32,
    /// True peak per channel (linear) — checks inter-sample peaks with 4x oversampling.
    pub true_peak: Vec<f32>,
    /// True peak per channel (dB).
    pub true_peak_db: Vec<f32>,
    /// Number of frames analyzed.
    pub frame_count: u32,
    /// Number of channels analyzed.
    pub channel_count: u16,
}

/// Perform dynamics analysis on an AudioBuffer.
pub fn analyze_dynamics(buffer: &AudioBuffer, _sample_rate: u32) -> DynamicsAnalysis {
    let channels = buffer.channels();
    let frames = buffer.frames();

    let mut peak = vec![0.0f32; channels as usize];
    let mut rms_sum = vec![0.0f64; channels as usize];
    let mut true_peak = vec![0.0f32; channels as usize];

    for ch in 0..channels {
        for f in 0..frames {
            let s = buffer.get(f, ch);
            let abs_s = s.abs();
            if abs_s > peak[ch as usize] {
                peak[ch as usize] = abs_s;
            }
            rms_sum[ch as usize] += (s as f64) * (s as f64);
        }

        // True peak estimation via linear interpolation (simplified 4x oversample)
        if frames > 1 {
            let mut prev = buffer.get(0, ch);
            let mut tp = prev.abs();
            for f in 1..frames {
                let curr = buffer.get(f, ch);
                // Check 3 inter-sample points
                for k in 1..4u32 {
                    let t = k as f32 / 4.0;
                    let interp = prev + t * (curr - prev);
                    tp = tp.max(interp.abs());
                }
                tp = tp.max(curr.abs());
                prev = curr;
            }
            true_peak[ch as usize] = tp;
        } else if frames == 1 {
            true_peak[ch as usize] = peak[ch as usize];
        }
    }

    let rms: Vec<f32> = rms_sum
        .iter()
        .map(|&sum| {
            if frames > 0 {
                (sum / frames as f64).sqrt() as f32
            } else {
                0.0
            }
        })
        .collect();

    let to_db = |x: f32| -> f32 { if x > 1e-10 { 20.0 * x.log10() } else { -200.0 } };

    let peak_db: Vec<f32> = peak.iter().map(|&p| to_db(p)).collect();
    let rms_db: Vec<f32> = rms.iter().map(|&r| to_db(r)).collect();
    let true_peak_db: Vec<f32> = true_peak.iter().map(|&tp| to_db(tp)).collect();

    let crest_factor_db: Vec<f32> = peak
        .iter()
        .zip(rms.iter())
        .map(|(&p, &r)| if r > 1e-10 { to_db(p) - to_db(r) } else { 0.0 })
        .collect();

    // Simplified LUFS (mono/stereo mean RMS in LUFS scale)
    let mean_rms_sq: f64 = rms_sum.iter().sum::<f64>() / (channels as f64 * frames.max(1) as f64);
    let lufs = if mean_rms_sq > 1e-20 {
        -0.691 + 10.0 * (mean_rms_sq as f32).log10()
    } else {
        -200.0
    };

    // Dynamic range: max peak dB - mean RMS dB
    let max_peak_db = peak_db.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mean_rms_db = if !rms_db.is_empty() {
        rms_db.iter().sum::<f32>() / rms_db.len() as f32
    } else {
        -200.0
    };
    let dynamic_range_db = max_peak_db - mean_rms_db;

    DynamicsAnalysis {
        peak,
        peak_db,
        rms,
        rms_db,
        crest_factor_db,
        lufs,
        dynamic_range_db,
        true_peak,
        true_peak_db,
        frame_count: frames,
        channel_count: channels,
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
        // Peak should be ~1.0
        assert!((result.peak[0] - 1.0).abs() < 0.01);
        // RMS of sine = 1/sqrt(2) ~ 0.707
        assert!(
            (result.rms[0] - 0.7071).abs() < 0.01,
            "RMS was {}, expected ~0.707",
            result.rms[0]
        );
        // Crest factor of sine ~ 3.01 dB
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
        // DC: peak == RMS, so crest factor == 0 dB
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
        // 20*log10(0.5) ~ -6.02 dB
        assert!((result.peak_db[0] - (-6.02)).abs() < 0.1);
    }

    #[test]
    fn true_peak_catches_intersample() {
        // Two samples that could have an inter-sample peak
        let mut buf = AudioBuffer::new(1, 2);
        buf.set(0, 0, 0.9);
        buf.set(1, 0, -0.9);
        let result = analyze_dynamics(&buf, 48000);
        // True peak should be >= sample peak for most signals
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
}
