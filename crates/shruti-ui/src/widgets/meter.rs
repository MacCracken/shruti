use egui::{Rect, Ui, Widget, pos2, vec2};

use crate::theme::ThemeColors;

/// Vertical level meter with peak + RMS display.
pub struct LevelMeter<'a> {
    peak_l: f32,
    peak_r: f32,
    rms_l: f32,
    rms_r: f32,
    colors: &'a ThemeColors,
    height: f32,
}

impl<'a> LevelMeter<'a> {
    pub fn new(peak_l: f32, peak_r: f32, rms_l: f32, rms_r: f32, colors: &'a ThemeColors) -> Self {
        Self {
            peak_l,
            peak_r,
            rms_l,
            rms_r,
            colors,
            height: 160.0,
        }
    }

    pub fn stereo(peak: [f32; 2], rms: [f32; 2], colors: &'a ThemeColors) -> Self {
        Self::new(peak[0], peak[1], rms[0], rms[1], colors)
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    fn db_to_normalized(db: f32) -> f32 {
        // Map -60dB..+6dB to 0..1
        ((db + 60.0) / 66.0).clamp(0.0, 1.0)
    }

    fn linear_to_db(linear: f32) -> f32 {
        if linear < 1e-10 {
            -60.0
        } else {
            20.0 * linear.log10()
        }
    }

    fn meter_color(&self, normalized: f32) -> egui::Color32 {
        if normalized > 0.91 {
            // > -0.5 dB
            self.colors.meter_red()
        } else if normalized > 0.73 {
            // > -6 dB
            self.colors.meter_yellow()
        } else {
            self.colors.meter_green()
        }
    }

    fn draw_bar(&self, ui: &mut Ui, rect: Rect, peak_linear: f32, rms_linear: f32) {
        let painter = ui.painter_at(rect);

        // Background
        painter.rect_filled(rect, 1.0, self.colors.bg_primary());

        let peak_db = Self::linear_to_db(peak_linear);
        let rms_db = Self::linear_to_db(rms_linear);
        let peak_n = Self::db_to_normalized(peak_db);
        let rms_n = Self::db_to_normalized(rms_db);

        // RMS fill (wider, dimmer)
        let rms_h = rms_n * rect.height();
        if rms_h > 0.5 {
            let rms_rect = Rect::from_min_max(
                pos2(rect.left(), rect.bottom() - rms_h),
                rect.right_bottom(),
            );
            let color = self.meter_color(rms_n).linear_multiply(0.5);
            painter.rect_filled(rms_rect, 0.0, color);
        }

        // Peak fill (brighter, on top)
        let peak_h = peak_n * rect.height();
        if peak_h > 0.5 {
            let peak_rect = Rect::from_min_max(
                pos2(rect.left() + 1.0, rect.bottom() - peak_h),
                pos2(rect.right() - 1.0, rect.bottom()),
            );
            let color = self.meter_color(peak_n);
            painter.rect_filled(peak_rect, 0.0, color);
        }

        // Peak hold line
        if peak_n > 0.01 {
            let y = rect.bottom() - peak_n * rect.height();
            painter.line_segment(
                [pos2(rect.left(), y), pos2(rect.right(), y)],
                egui::Stroke::new(1.0, self.meter_color(peak_n)),
            );
        }
    }
}

impl Widget for LevelMeter<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let bar_width = 6.0;
        let gap = 2.0;
        let total_width = bar_width * 2.0 + gap;
        let desired_size = vec2(total_width, self.height);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let left_rect = Rect::from_min_size(rect.min, vec2(bar_width, self.height));
            let right_rect = Rect::from_min_size(
                pos2(rect.left() + bar_width + gap, rect.top()),
                vec2(bar_width, self.height),
            );

            self.draw_bar(ui, left_rect, self.peak_l, self.rms_l);
            self.draw_bar(ui, right_rect, self.peak_r, self.rms_r);
        }

        response
    }
}
