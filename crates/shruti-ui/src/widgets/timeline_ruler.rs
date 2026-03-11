use egui::{Rect, Stroke, Ui, pos2};

use crate::theme::ThemeColors;

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

    let frames_per_beat = (sample_rate as f64 * 60.0) / bpm;
    let frames_per_bar = frames_per_beat * 4.0; // Assuming 4/4
    let pixels_per_bar = frames_per_bar * pixels_per_frame;

    // Determine tick spacing based on zoom
    let (major_interval, minor_interval, label_fn): (f64, f64, Box<dyn Fn(i64) -> String>) =
        if pixels_per_bar > 200.0 {
            // Show beats
            (
                frames_per_bar,
                frames_per_beat,
                Box::new(move |frame: i64| {
                    let bar = (frame as f64 / frames_per_bar).floor() as i64 + 1;
                    format!("{bar}")
                }),
            )
        } else if pixels_per_bar > 40.0 {
            // Show bars
            (
                frames_per_bar,
                frames_per_bar,
                Box::new(move |frame: i64| {
                    let bar = (frame as f64 / frames_per_bar).floor() as i64 + 1;
                    format!("{bar}")
                }),
            )
        } else {
            // Show every N bars
            let bar_skip = (80.0 / pixels_per_bar).ceil() as i64;
            let major = frames_per_bar * bar_skip as f64;
            (
                major,
                frames_per_bar,
                Box::new(move |frame: i64| {
                    let bar = (frame as f64 / frames_per_bar).floor() as i64 + 1;
                    format!("{bar}")
                }),
            )
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
