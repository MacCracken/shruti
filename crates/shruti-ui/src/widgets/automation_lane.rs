use egui::{Rect, Stroke, Ui, pos2};

use crate::theme::ThemeColors;

/// Convert a frame position to a pixel X coordinate within a rect.
pub(crate) fn frame_to_x(
    frame: f64,
    rect_left: f32,
    pixels_per_frame: f64,
    scroll_offset: f64,
) -> f32 {
    rect_left + (frame * pixels_per_frame - scroll_offset) as f32
}

/// Convert a normalised automation value (0..1) to a pixel Y coordinate within a rect.
pub(crate) fn value_to_y(value: f32, rect_bottom: f32, rect_height: f32) -> f32 {
    rect_bottom - value.clamp(0.0, 1.0) * rect_height
}

/// Linearly interpolate between two automation values at parameter `t` in 0..1.
pub(crate) fn lerp_value(v1: f32, v2: f32, t: f32) -> f32 {
    v1 + (v2 - v1) * t
}

/// Draw automation points and curves on a lane overlay.
pub fn draw_automation(
    ui: &mut Ui,
    rect: Rect,
    points: &[(f64, f32)], // (frame position, value 0..1)
    scroll_offset: f64,
    pixels_per_frame: f64,
    colors: &ThemeColors,
) {
    if points.is_empty() || rect.width() < 1.0 {
        return;
    }

    let painter = ui.painter_at(rect);

    let ftx =
        |frame: f64| -> f32 { frame_to_x(frame, rect.left(), pixels_per_frame, scroll_offset) };

    let vty = |value: f32| -> f32 { value_to_y(value, rect.bottom(), rect.height()) };

    // Draw curve segments between points
    let curve_stroke = Stroke::new(1.5, colors.automation_curve());

    for i in 0..points.len().saturating_sub(1) {
        let (f1, v1) = points[i];
        let (f2, v2) = points[i + 1];

        let x1 = ftx(f1);
        let x2 = ftx(f2);

        // Skip if completely off-screen
        if x2 < rect.left() || x1 > rect.right() {
            continue;
        }

        // Linear interpolation as line segments
        let steps = ((x2 - x1).abs() as usize).clamp(1, 200);
        for s in 0..steps {
            let t1 = s as f32 / steps as f32;
            let t2 = (s + 1) as f32 / steps as f32;
            let y1 = vty(lerp_value(v1, v2, t1));
            let y2 = vty(lerp_value(v1, v2, t2));
            let sx1 = x1 + (x2 - x1) * t1;
            let sx2 = x1 + (x2 - x1) * t2;

            painter.line_segment([pos2(sx1, y1), pos2(sx2, y2)], curve_stroke);
        }
    }

    // Draw points
    for &(frame, value) in points {
        let x = ftx(frame);
        let y = vty(value);

        if x >= rect.left() - 5.0 && x <= rect.right() + 5.0 {
            painter.circle_filled(pos2(x, y), 4.0, colors.automation_point());
            painter.circle_stroke(pos2(x, y), 4.0, Stroke::new(1.0, colors.automation_curve()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- frame_to_x tests ---

    #[test]
    fn frame_to_x_at_origin_no_scroll() {
        // Frame 0, rect starts at 10.0, no scroll
        let x = frame_to_x(0.0, 10.0, 0.01, 0.0);
        assert!((x - 10.0).abs() < 0.001);
    }

    #[test]
    fn frame_to_x_with_offset() {
        // Frame 1000 at 0.01 ppf => 10px offset from rect_left
        let x = frame_to_x(1000.0, 0.0, 0.01, 0.0);
        assert!((x - 10.0).abs() < 0.001);
    }

    #[test]
    fn frame_to_x_with_scroll() {
        // Frame 1000 at 0.01 ppf with 5px scroll => 10 - 5 = 5
        let x = frame_to_x(1000.0, 0.0, 0.01, 5.0);
        assert!((x - 5.0).abs() < 0.001);
    }

    #[test]
    fn frame_to_x_rect_left_offset() {
        // rect_left=50, frame 0, no scroll => 50
        let x = frame_to_x(0.0, 50.0, 0.01, 0.0);
        assert!((x - 50.0).abs() < 0.001);
    }

    #[test]
    fn frame_to_x_large_frame() {
        let x = frame_to_x(48000.0, 0.0, 0.01, 0.0);
        assert!((x - 480.0).abs() < 0.01);
    }

    // --- value_to_y tests ---

    #[test]
    fn value_to_y_at_zero_is_bottom() {
        // value=0 => y = rect_bottom - 0 * height = bottom
        let y = value_to_y(0.0, 200.0, 100.0);
        assert!((y - 200.0).abs() < 0.001);
    }

    #[test]
    fn value_to_y_at_one_is_top() {
        // value=1 => y = rect_bottom - 1 * height = 200 - 100 = 100
        let y = value_to_y(1.0, 200.0, 100.0);
        assert!((y - 100.0).abs() < 0.001);
    }

    #[test]
    fn value_to_y_at_half() {
        // value=0.5 => y = 200 - 0.5 * 100 = 150
        let y = value_to_y(0.5, 200.0, 100.0);
        assert!((y - 150.0).abs() < 0.001);
    }

    #[test]
    fn value_to_y_clamps_above_one() {
        // value=1.5 clamped to 1.0 => y = 200 - 100 = 100
        let y = value_to_y(1.5, 200.0, 100.0);
        assert!((y - 100.0).abs() < 0.001);
    }

    #[test]
    fn value_to_y_clamps_below_zero() {
        // value=-0.5 clamped to 0.0 => y = 200
        let y = value_to_y(-0.5, 200.0, 100.0);
        assert!((y - 200.0).abs() < 0.001);
    }

    // --- lerp_value tests ---

    #[test]
    fn lerp_at_zero() {
        assert!((lerp_value(0.0, 1.0, 0.0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lerp_at_one() {
        assert!((lerp_value(0.0, 1.0, 1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lerp_at_half() {
        assert!((lerp_value(0.0, 1.0, 0.5) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn lerp_negative_range() {
        // -1 to 1 at t=0.5 => 0
        assert!((lerp_value(-1.0, 1.0, 0.5) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lerp_same_values() {
        assert!((lerp_value(0.5, 0.5, 0.7) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn lerp_quarter() {
        // 0 to 100 at t=0.25 => 25
        assert!((lerp_value(0.0, 100.0, 0.25) - 25.0).abs() < 0.001);
    }

    // --- integration: frame_to_x + value_to_y pipeline ---

    #[test]
    fn automation_point_at_origin() {
        let x = frame_to_x(0.0, 100.0, 0.01, 0.0);
        let y = value_to_y(0.0, 300.0, 200.0);
        assert!((x - 100.0).abs() < 0.001);
        assert!((y - 300.0).abs() < 0.001);
    }

    #[test]
    fn automation_point_midway() {
        let x = frame_to_x(5000.0, 100.0, 0.01, 0.0);
        let y = value_to_y(0.5, 300.0, 200.0);
        // x = 100 + 50 = 150
        assert!((x - 150.0).abs() < 0.001);
        // y = 300 - 100 = 200
        assert!((y - 200.0).abs() < 0.001);
    }
}
