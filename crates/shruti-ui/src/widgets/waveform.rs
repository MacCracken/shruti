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
