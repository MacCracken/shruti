use egui::{Color32, Rect, Response, Sense, Stroke, Ui, Widget, pos2, vec2};

use crate::theme::ThemeColors;

/// Vertical fader widget for gain control.
pub struct Fader<'a> {
    value: &'a mut f32,
    min_db: f32,
    max_db: f32,
    colors: &'a ThemeColors,
    height: f32,
    width: f32,
}

impl<'a> Fader<'a> {
    pub fn new(value: &'a mut f32, colors: &'a ThemeColors) -> Self {
        Self {
            value,
            min_db: -60.0,
            max_db: 12.0,
            colors,
            height: 160.0,
            width: 24.0,
        }
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    fn db_to_normalized(&self, db: f32) -> f32 {
        (db - self.min_db) / (self.max_db - self.min_db)
    }

    fn linear_to_db(linear: f32) -> f32 {
        if linear < 1e-10 {
            -60.0
        } else {
            20.0 * linear.log10()
        }
    }

    fn db_to_linear(db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }
}

impl Widget for Fader<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = vec2(self.width, self.height);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        let current_db = Self::linear_to_db(*self.value);

        // Handle drag interaction
        if response.dragged() {
            let delta_normalized = -response.drag_delta().y / self.height;
            let new_db = (current_db + delta_normalized * (self.max_db - self.min_db))
                .clamp(self.min_db, self.max_db);
            *self.value = Self::db_to_linear(new_db);
        }

        // Double-click to reset to unity
        if response.double_clicked() {
            *self.value = 1.0;
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);

            // Track groove
            let groove_x = rect.center().x;
            let groove_rect = Rect::from_min_max(
                pos2(groove_x - 2.0, rect.top() + 4.0),
                pos2(groove_x + 2.0, rect.bottom() - 4.0),
            );
            painter.rect_filled(groove_rect, 2.0, self.colors.bg_primary());

            // Unity (0dB) mark
            let unity_y = rect.bottom() - 4.0 - self.db_to_normalized(0.0) * (self.height - 8.0);
            painter.line_segment(
                [
                    pos2(rect.left() + 2.0, unity_y),
                    pos2(rect.right() - 2.0, unity_y),
                ],
                Stroke::new(1.0, self.colors.text_secondary()),
            );

            // dB scale marks
            for &db in &[-48.0, -24.0, -12.0, -6.0, 6.0] {
                let y = rect.bottom() - 4.0 - self.db_to_normalized(db) * (self.height - 8.0);
                painter.line_segment(
                    [pos2(groove_x - 4.0, y), pos2(groove_x + 4.0, y)],
                    Stroke::new(0.5, self.colors.separator()),
                );
            }

            // Fader cap
            let normalized = self.db_to_normalized(current_db.clamp(self.min_db, self.max_db));
            let fader_y = rect.bottom() - 4.0 - normalized * (self.height - 8.0);
            let cap_rect =
                Rect::from_center_size(pos2(groove_x, fader_y), vec2(self.width - 4.0, 10.0));

            let cap_color = if response.hovered() || response.dragged() {
                self.colors.accent()
            } else {
                self.colors.surface_hover()
            };
            painter.rect_filled(cap_rect, 3.0, cap_color);
            // Center line on cap
            painter.line_segment(
                [
                    pos2(cap_rect.left() + 3.0, fader_y),
                    pos2(cap_rect.right() - 3.0, fader_y),
                ],
                Stroke::new(1.0, Color32::WHITE.linear_multiply(0.6)),
            );

            // Fill below cap
            let fill_rect = Rect::from_min_max(
                pos2(groove_x - 2.0, fader_y),
                pos2(groove_x + 2.0, rect.bottom() - 4.0),
            );
            painter.rect_filled(fill_rect, 1.0, self.colors.accent().linear_multiply(0.5));
        }

        response.on_hover_text(format!("{:.1} dB", Self::linear_to_db(*self.value)))
    }
}
