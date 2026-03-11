use egui::{CornerRadius, Rect, Stroke, Ui, pos2, vec2};

use crate::theme::ThemeColors;
use crate::widgets::track_header::track_color;

/// Draw a region/clip rectangle on the timeline.
pub fn draw_region(
    ui: &mut Ui,
    rect: Rect,
    name: &str,
    color_index: usize,
    selected: bool,
    colors: &ThemeColors,
) {
    if rect.width() < 1.0 {
        return;
    }

    let painter = ui.painter_at(rect);
    let rounding = CornerRadius::same(3);

    // Region background
    let base_color = track_color(color_index);
    let bg = if selected {
        colors.region_selected()
    } else {
        base_color.linear_multiply(0.3)
    };
    painter.rect_filled(rect, rounding, bg);

    // Border
    let border_color = if selected {
        colors.accent()
    } else {
        base_color.linear_multiply(0.6)
    };
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0, border_color),
        egui::StrokeKind::Outside,
    );

    // Top accent bar
    let top_bar = Rect::from_min_size(rect.min, vec2(rect.width(), 2.0));
    painter.rect_filled(top_bar, CornerRadius::ZERO, base_color.linear_multiply(0.7));

    // Name label (clipped to region bounds)
    if rect.width() > 30.0 {
        let text_rect = rect.shrink2(vec2(4.0, 2.0));
        let text_pos = pos2(text_rect.left(), text_rect.top() + 2.0);
        painter.text(
            text_pos,
            egui::Align2::LEFT_TOP,
            name,
            egui::FontId::new(9.0, egui::FontFamily::Proportional),
            colors.text_primary(),
        );
    }
}
