use egui::{Rect, Stroke, Ui, pos2};

use crate::theme::ThemeColors;

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

    let frame_to_x =
        |frame: f64| -> f32 { rect.left() + (frame * pixels_per_frame - scroll_offset) as f32 };

    let value_to_y = |value: f32| -> f32 { rect.bottom() - value.clamp(0.0, 1.0) * rect.height() };

    // Draw curve segments between points
    let curve_stroke = Stroke::new(1.5, colors.automation_curve());

    for i in 0..points.len().saturating_sub(1) {
        let (f1, v1) = points[i];
        let (f2, v2) = points[i + 1];

        let x1 = frame_to_x(f1);
        let x2 = frame_to_x(f2);

        // Skip if completely off-screen
        if x2 < rect.left() || x1 > rect.right() {
            continue;
        }

        // Linear interpolation as line segments
        let steps = ((x2 - x1).abs() as usize).clamp(1, 200);
        for s in 0..steps {
            let t1 = s as f32 / steps as f32;
            let t2 = (s + 1) as f32 / steps as f32;
            let y1 = value_to_y(v1 + (v2 - v1) * t1);
            let y2 = value_to_y(v1 + (v2 - v1) * t2);
            let sx1 = x1 + (x2 - x1) * t1;
            let sx2 = x1 + (x2 - x1) * t2;

            painter.line_segment([pos2(sx1, y1), pos2(sx2, y2)], curve_stroke);
        }
    }

    // Draw points
    for &(frame, value) in points {
        let x = frame_to_x(frame);
        let y = value_to_y(value);

        if x >= rect.left() - 5.0 && x <= rect.right() + 5.0 {
            painter.circle_filled(pos2(x, y), 4.0, colors.automation_point());
            painter.circle_stroke(pos2(x, y), 4.0, Stroke::new(1.0, colors.automation_curve()));
        }
    }
}
