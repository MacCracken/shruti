use egui::{Color32, Rect, Stroke, Ui, pos2};

use crate::theme::ThemeColors;

/// Pre-computed waveform peak data for display at different zoom levels.
#[derive(Debug, Clone)]
pub struct WaveformPeaks {
    /// Min/max pairs per pixel column at various resolutions.
    /// Index 0 = finest (1:1), each subsequent level halves resolution.
    pub levels: Vec<Vec<(f32, f32)>>,
}

impl WaveformPeaks {
    /// Compute mipmap-style peak data from raw samples.
    pub fn from_samples(samples: &[f32], channel: usize, channels: usize) -> Self {
        // Extract mono channel
        let mono: Vec<f32> = samples
            .iter()
            .skip(channel)
            .step_by(channels)
            .copied()
            .collect();

        let mut levels = Vec::new();

        // Level 0: raw peaks per sample
        let level0: Vec<(f32, f32)> = mono.iter().map(|&s| (s.min(0.0), s.max(0.0))).collect();
        levels.push(level0);

        // Generate mipmap levels (each halving)
        let mut current = &levels[0];
        for _ in 0..12 {
            if current.len() <= 1 {
                break;
            }
            let next: Vec<(f32, f32)> = current
                .chunks(2)
                .map(|chunk| {
                    let min = chunk.iter().map(|p| p.0).fold(f32::MAX, f32::min);
                    let max = chunk.iter().map(|p| p.1).fold(f32::MIN, f32::max);
                    (min, max)
                })
                .collect();
            levels.push(next);
            current = levels.last().unwrap();
        }

        Self { levels }
    }

    /// Get the best mipmap level for the given samples-per-pixel ratio.
    pub fn peaks_for_zoom(&self, samples_per_pixel: f32) -> &[(f32, f32)] {
        let level = (samples_per_pixel.log2().max(0.0)) as usize;
        let idx = level.min(self.levels.len() - 1);
        &self.levels[idx]
    }
}

/// Draw a waveform into a given rectangle.
pub fn draw_waveform(
    ui: &mut Ui,
    rect: Rect,
    peaks: &WaveformPeaks,
    start_sample: usize,
    samples_per_pixel: f32,
    colors: &ThemeColors,
) {
    if rect.width() < 1.0 || rect.height() < 1.0 || peaks.levels.is_empty() {
        return;
    }

    let painter = ui.painter_at(rect);
    let center_y = rect.center().y;
    let half_h = rect.height() / 2.0;

    let peak_data = peaks.peaks_for_zoom(samples_per_pixel);
    let zoom_divisor = (samples_per_pixel as usize).max(1);

    let fill_color = colors.waveform();
    let outline_color = colors.waveform_outline();

    let width = rect.width() as usize;
    for px in 0..width {
        let sample_idx = start_sample + px * zoom_divisor;
        let peak_idx = sample_idx / zoom_divisor.max(1);

        if peak_idx >= peak_data.len() {
            break;
        }

        let (min, max) = peak_data[peak_idx.min(peak_data.len() - 1)];
        let x = rect.left() + px as f32;
        let y_top = center_y - max * half_h;
        let y_bot = center_y - min * half_h;

        // Fill
        if (y_bot - y_top).abs() > 0.5 {
            painter.line_segment(
                [pos2(x, y_top), pos2(x, y_bot)],
                Stroke::new(1.0, fill_color),
            );
        } else {
            // Single pixel dot
            painter.line_segment(
                [pos2(x, center_y - 0.5), pos2(x, center_y + 0.5)],
                Stroke::new(1.0, fill_color),
            );
        }

        // Outline at peaks
        painter.circle_filled(pos2(x, y_top), 0.3, outline_color);
        painter.circle_filled(pos2(x, y_bot), 0.3, outline_color);
    }

    // Center line
    painter.line_segment(
        [pos2(rect.left(), center_y), pos2(rect.right(), center_y)],
        Stroke::new(0.5, Color32::from_white_alpha(30)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_samples_mono_basic() {
        // Mono signal: [0.5, -0.3, 0.8, -0.1]
        let samples = vec![0.5_f32, -0.3, 0.8, -0.1];
        let peaks = WaveformPeaks::from_samples(&samples, 0, 1);

        assert!(!peaks.levels.is_empty());
        // Level 0 should have 4 entries (one per sample)
        assert_eq!(peaks.levels[0].len(), 4);

        // Positive samples: min=0, max=sample
        assert_eq!(peaks.levels[0][0], (0.0, 0.5));
        // Negative samples: min=sample, max=0
        assert_eq!(peaks.levels[0][1], (-0.3, 0.0));
        assert_eq!(peaks.levels[0][2], (0.0, 0.8));
        assert_eq!(peaks.levels[0][3], (-0.1, 0.0));
    }

    #[test]
    fn from_samples_stereo_extracts_channel() {
        // Interleaved stereo: L=0.5, R=-0.5, L=0.3, R=-0.3
        let samples = vec![0.5_f32, -0.5, 0.3, -0.3];
        let left = WaveformPeaks::from_samples(&samples, 0, 2);
        let right = WaveformPeaks::from_samples(&samples, 1, 2);

        assert_eq!(left.levels[0].len(), 2);
        assert_eq!(left.levels[0][0], (0.0, 0.5));
        assert_eq!(left.levels[0][1], (0.0, 0.3));

        assert_eq!(right.levels[0].len(), 2);
        assert_eq!(right.levels[0][0], (-0.5, 0.0));
        assert_eq!(right.levels[0][1], (-0.3, 0.0));
    }

    #[test]
    fn mipmap_levels_halve_in_size() {
        let samples: Vec<f32> = (0..64).map(|i| (i as f32 / 32.0) - 1.0).collect();
        let peaks = WaveformPeaks::from_samples(&samples, 0, 1);

        assert!(peaks.levels.len() >= 3);
        assert_eq!(peaks.levels[0].len(), 64);
        assert_eq!(peaks.levels[1].len(), 32);
        assert_eq!(peaks.levels[2].len(), 16);

        // Each subsequent level should be half
        for i in 1..peaks.levels.len() {
            let expected = peaks.levels[i - 1].len().div_ceil(2);
            assert_eq!(peaks.levels[i].len(), expected);
        }
    }

    #[test]
    fn mipmap_preserves_min_max() {
        // Two samples: 0.5 and -0.3; level 1 should merge them
        let samples = vec![0.5_f32, -0.3];
        let peaks = WaveformPeaks::from_samples(&samples, 0, 1);

        assert!(peaks.levels.len() >= 2);
        // Level 1 merges the two: min of (0.0, -0.3) = -0.3, max of (0.5, 0.0) = 0.5
        assert_eq!(peaks.levels[1][0], (-0.3, 0.5));
    }

    #[test]
    fn peaks_for_zoom_level_0() {
        let samples: Vec<f32> = (0..16).map(|i| (i as f32) / 16.0).collect();
        let peaks = WaveformPeaks::from_samples(&samples, 0, 1);

        // samples_per_pixel = 1.0 => log2(1) = 0 => level 0
        let data = peaks.peaks_for_zoom(1.0);
        assert_eq!(data.len(), 16);
    }

    #[test]
    fn peaks_for_zoom_higher_levels() {
        let samples: Vec<f32> = (0..64).map(|i| (i as f32) / 64.0).collect();
        let peaks = WaveformPeaks::from_samples(&samples, 0, 1);

        // samples_per_pixel = 2.0 => log2(2) = 1 => level 1
        let level1 = peaks.peaks_for_zoom(2.0);
        assert_eq!(level1.len(), 32);

        // samples_per_pixel = 4.0 => log2(4) = 2 => level 2
        let level2 = peaks.peaks_for_zoom(4.0);
        assert_eq!(level2.len(), 16);
    }

    #[test]
    fn peaks_for_zoom_clamps_to_max_level() {
        let samples = vec![0.5_f32; 4];
        let peaks = WaveformPeaks::from_samples(&samples, 0, 1);
        let max_level = peaks.levels.len() - 1;

        // Very high zoom should clamp to last level
        let data = peaks.peaks_for_zoom(100000.0);
        assert_eq!(data.len(), peaks.levels[max_level].len());
    }

    #[test]
    fn empty_samples() {
        let peaks = WaveformPeaks::from_samples(&[], 0, 1);
        assert_eq!(peaks.levels.len(), 1);
        assert!(peaks.levels[0].is_empty());
    }

    #[test]
    fn single_sample() {
        let peaks = WaveformPeaks::from_samples(&[0.7], 0, 1);
        assert_eq!(peaks.levels[0].len(), 1);
        assert_eq!(peaks.levels[0][0], (0.0, 0.7));
    }
}
