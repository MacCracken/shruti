use egui::{Rect, Stroke, Ui, pos2};

use crate::theme::ThemeColors;

/// Grid interval classification based on zoom level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RulerGridMode {
    /// Zoomed in enough to show individual beats within bars.
    Beats,
    /// Medium zoom: show each bar, no beat subdivisions.
    Bars,
    /// Zoomed out: skip bars, showing every Nth bar.
    SkipBars {
        /// Number of bars between major ticks.
        bar_skip: i64,
    },
}

/// Determine the grid mode (beat/bar/skip) based on zoom level.
///
/// Returns the mode and the computed `frames_per_bar` and `frames_per_beat`.
pub(crate) fn determine_grid_mode(
    pixels_per_frame: f64,
    sample_rate: u32,
    bpm: f64,
) -> (RulerGridMode, f64, f64) {
    let frames_per_beat = (sample_rate as f64 * 60.0) / bpm;
    let frames_per_bar = frames_per_beat * 4.0;
    let pixels_per_bar = frames_per_bar * pixels_per_frame;

    let mode = if pixels_per_bar > 200.0 {
        RulerGridMode::Beats
    } else if pixels_per_bar > 40.0 {
        RulerGridMode::Bars
    } else {
        let bar_skip = (80.0 / pixels_per_bar).ceil() as i64;
        RulerGridMode::SkipBars { bar_skip }
    };

    (mode, frames_per_bar, frames_per_beat)
}

/// Draw a timeline ruler showing bars/beats or time.
pub fn draw_ruler(
    ui: &mut Ui,
    rect: Rect,
    scroll_offset: f64,
    pixels_per_frame: f64,
    sample_rate: u32,
    bpm: f64,
    colors: &ThemeColors,
) {
    let painter = ui.painter_at(rect);

    // Background
    painter.rect_filled(rect, 0.0, colors.bg_secondary());

    // Bottom border
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, colors.separator()),
    );

    let (mode, frames_per_bar, frames_per_beat) =
        determine_grid_mode(pixels_per_frame, sample_rate, bpm);

    // Determine tick spacing based on zoom
    let (major_interval, minor_interval): (f64, f64) = match mode {
        RulerGridMode::Beats => (frames_per_bar, frames_per_beat),
        RulerGridMode::Bars => (frames_per_bar, frames_per_bar),
        RulerGridMode::SkipBars { bar_skip } => (frames_per_bar * bar_skip as f64, frames_per_bar),
    };
    let label_fn = |frame: i64| -> String {
        let bar = (frame as f64 / frames_per_bar).floor() as i64 + 1;
        format!("{bar}")
    };

    // Visible frame range
    let start_frame = (scroll_offset / pixels_per_frame) as i64;
    let end_frame = start_frame + (rect.width() as f64 / pixels_per_frame) as i64;

    // Minor ticks
    let minor_start = (start_frame as f64 / minor_interval).floor() as i64;
    let minor_end = (end_frame as f64 / minor_interval).ceil() as i64;

    for i in minor_start..=minor_end {
        let frame = (i as f64 * minor_interval) as i64;
        let x = rect.left() + (frame as f64 * pixels_per_frame - scroll_offset) as f32;
        if x >= rect.left() && x <= rect.right() {
            painter.line_segment(
                [pos2(x, rect.bottom() - 4.0), pos2(x, rect.bottom())],
                Stroke::new(0.5, colors.text_secondary()),
            );
        }
    }

    // Major ticks with labels
    let major_start = (start_frame as f64 / major_interval).floor() as i64;
    let major_end = (end_frame as f64 / major_interval).ceil() as i64;

    for i in major_start..=major_end {
        let frame = (i as f64 * major_interval) as i64;
        let x = rect.left() + (frame as f64 * pixels_per_frame - scroll_offset) as f32;
        if x >= rect.left() && x <= rect.right() {
            painter.line_segment(
                [pos2(x, rect.bottom() - 10.0), pos2(x, rect.bottom())],
                Stroke::new(1.0, colors.text_secondary()),
            );

            let label = label_fn(frame);
            painter.text(
                pos2(x + 3.0, rect.top() + 2.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::new(9.0, egui::FontFamily::Proportional),
                colors.text_secondary(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- determine_grid_mode tests ----

    #[test]
    fn grid_mode_beats_at_high_zoom() {
        // 120 BPM, 48kHz => frames_per_beat=24000, frames_per_bar=96000
        // pixels_per_bar = 96000 * 0.01 = 960 > 200 => Beats
        let (mode, fpbar, fpbeat) = determine_grid_mode(0.01, 48000, 120.0);
        assert_eq!(mode, RulerGridMode::Beats);
        assert!((fpbar - 96000.0).abs() < 1.0);
        assert!((fpbeat - 24000.0).abs() < 1.0);
    }

    #[test]
    fn grid_mode_bars_at_medium_zoom() {
        // pixels_per_bar = 96000 * 0.001 = 96 => between 40 and 200 => Bars
        let (mode, _, _) = determine_grid_mode(0.001, 48000, 120.0);
        assert_eq!(mode, RulerGridMode::Bars);
    }

    #[test]
    fn grid_mode_skip_bars_at_low_zoom() {
        // pixels_per_bar = 96000 * 0.0001 = 9.6 => < 40 => SkipBars
        let (mode, _, _) = determine_grid_mode(0.0001, 48000, 120.0);
        match mode {
            RulerGridMode::SkipBars { bar_skip } => {
                assert!(bar_skip >= 1);
                // bar_skip = ceil(80 / 9.6) = 9
                assert_eq!(bar_skip, 9);
            }
            _ => panic!("Expected SkipBars mode"),
        }
    }

    #[test]
    fn grid_mode_very_low_zoom_large_skip() {
        // pixels_per_bar = 96000 * 0.00001 = 0.96 => SkipBars with large skip
        let (mode, _, _) = determine_grid_mode(0.00001, 48000, 120.0);
        match mode {
            RulerGridMode::SkipBars { bar_skip } => {
                assert!(bar_skip > 10);
            }
            _ => panic!("Expected SkipBars mode"),
        }
    }

    #[test]
    fn grid_mode_60_bpm() {
        // 60 BPM => frames_per_beat=48000, frames_per_bar=192000
        // pixels_per_bar = 192000 * 0.002 = 384 > 200 => Beats
        let (mode, fpbar, fpbeat) = determine_grid_mode(0.002, 48000, 60.0);
        assert_eq!(mode, RulerGridMode::Beats);
        assert!((fpbar - 192000.0).abs() < 1.0);
        assert!((fpbeat - 48000.0).abs() < 1.0);
    }

    #[test]
    fn grid_mode_44100_sample_rate() {
        // 44100 Hz, 120 BPM => frames_per_beat=22050
        let (_, _, fpbeat) = determine_grid_mode(0.01, 44100, 120.0);
        assert!((fpbeat - 22050.0).abs() < 1.0);
    }

    #[test]
    fn grid_mode_boundary_200_pixels_per_bar() {
        // pixels_per_bar = exactly 200 => should be Bars (> 200 is Beats)
        // ppf = 200 / 96000 = 0.00208333...
        // pixels_per_bar = 96000 * 0.00208333 = 200.0 => not > 200, so Bars
        let ppf = 200.0 / 96000.0;
        let (mode, _, _) = determine_grid_mode(ppf, 48000, 120.0);
        assert_eq!(mode, RulerGridMode::Bars);
    }

    #[test]
    fn grid_mode_boundary_40_pixels_per_bar() {
        // pixels_per_bar = exactly 40 => should be Bars (> 40 is Bars)
        // ppf = 40 / 96000
        // pixels_per_bar = 96000 * ppf = 40.0 => not > 40, so SkipBars
        let ppf = 40.0 / 96000.0;
        let (mode, _, _) = determine_grid_mode(ppf, 48000, 120.0);
        match mode {
            RulerGridMode::SkipBars { .. } => {}
            _ => panic!("Expected SkipBars at boundary"),
        }
    }
}
