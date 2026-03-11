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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeColors;

    fn colors() -> ThemeColors {
        ThemeColors::default()
    }

    // --- db_to_normalized tests ---

    #[test]
    fn db_to_normalized_at_minus_60_is_zero() {
        assert!((LevelMeter::db_to_normalized(-60.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn db_to_normalized_at_plus_6_is_one() {
        assert!((LevelMeter::db_to_normalized(6.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn db_to_normalized_at_zero_db() {
        // (0 + 60) / 66 = 60/66 ≈ 0.909
        let n = LevelMeter::db_to_normalized(0.0);
        assert!((n - 60.0 / 66.0).abs() < 0.001, "0 dB normalized: {n}");
    }

    #[test]
    fn db_to_normalized_clamps_below() {
        assert_eq!(LevelMeter::db_to_normalized(-100.0), 0.0);
    }

    #[test]
    fn db_to_normalized_clamps_above() {
        assert_eq!(LevelMeter::db_to_normalized(20.0), 1.0);
    }

    #[test]
    fn db_to_normalized_minus_6_db() {
        // (-6 + 60) / 66 = 54/66 ≈ 0.818
        let n = LevelMeter::db_to_normalized(-6.0);
        assert!((n - 54.0 / 66.0).abs() < 0.001);
    }

    #[test]
    fn db_to_normalized_minus_12_db() {
        let n = LevelMeter::db_to_normalized(-12.0);
        assert!((n - 48.0 / 66.0).abs() < 0.001);
    }

    // --- linear_to_db tests ---

    #[test]
    fn linear_to_db_unity() {
        let db = LevelMeter::linear_to_db(1.0);
        assert!((db - 0.0).abs() < 0.001);
    }

    #[test]
    fn linear_to_db_zero() {
        assert_eq!(LevelMeter::linear_to_db(0.0), -60.0);
    }

    #[test]
    fn linear_to_db_tiny() {
        assert_eq!(LevelMeter::linear_to_db(1e-12), -60.0);
    }

    #[test]
    fn linear_to_db_half() {
        let db = LevelMeter::linear_to_db(0.5);
        assert!((db - (-6.0206)).abs() < 0.01);
    }

    // --- meter_color tests ---

    #[test]
    fn meter_color_low_level_is_green() {
        let c = colors();
        let meter = LevelMeter::new(0.0, 0.0, 0.0, 0.0, &c);
        assert_eq!(meter.meter_color(0.5), c.meter_green());
        assert_eq!(meter.meter_color(0.0), c.meter_green());
        assert_eq!(meter.meter_color(0.72), c.meter_green());
    }

    #[test]
    fn meter_color_mid_level_is_yellow() {
        let c = colors();
        let meter = LevelMeter::new(0.0, 0.0, 0.0, 0.0, &c);
        assert_eq!(meter.meter_color(0.74), c.meter_yellow());
        assert_eq!(meter.meter_color(0.80), c.meter_yellow());
        assert_eq!(meter.meter_color(0.90), c.meter_yellow());
    }

    #[test]
    fn meter_color_high_level_is_red() {
        let c = colors();
        let meter = LevelMeter::new(0.0, 0.0, 0.0, 0.0, &c);
        assert_eq!(meter.meter_color(0.92), c.meter_red());
        assert_eq!(meter.meter_color(1.0), c.meter_red());
    }

    #[test]
    fn meter_color_boundary_at_073() {
        let c = colors();
        let meter = LevelMeter::new(0.0, 0.0, 0.0, 0.0, &c);
        // 0.73 is NOT > 0.73, so it's green
        assert_eq!(meter.meter_color(0.73), c.meter_green());
        // 0.731 IS > 0.73, so it's yellow
        assert_eq!(meter.meter_color(0.731), c.meter_yellow());
    }

    #[test]
    fn meter_color_boundary_at_091() {
        let c = colors();
        let meter = LevelMeter::new(0.0, 0.0, 0.0, 0.0, &c);
        // 0.91 is NOT > 0.91, so it's yellow
        assert_eq!(meter.meter_color(0.91), c.meter_yellow());
        // 0.911 IS > 0.91, so it's red
        assert_eq!(meter.meter_color(0.911), c.meter_red());
    }

    // --- constructor tests ---

    #[test]
    fn stereo_constructor() {
        let c = colors();
        let meter = LevelMeter::stereo([0.5, 0.6], [0.3, 0.4], &c);
        assert_eq!(meter.peak_l, 0.5);
        assert_eq!(meter.peak_r, 0.6);
        assert_eq!(meter.rms_l, 0.3);
        assert_eq!(meter.rms_r, 0.4);
    }

    #[test]
    fn default_height() {
        let c = colors();
        let meter = LevelMeter::new(0.0, 0.0, 0.0, 0.0, &c);
        assert_eq!(meter.height, 160.0);
    }

    #[test]
    fn custom_height() {
        let c = colors();
        let meter = LevelMeter::new(0.0, 0.0, 0.0, 0.0, &c).height(200.0);
        assert_eq!(meter.height, 200.0);
    }

    // --- integration: linear -> db -> normalized pipeline ---

    #[test]
    fn full_pipeline_unity_signal() {
        let db = LevelMeter::linear_to_db(1.0);
        let n = LevelMeter::db_to_normalized(db);
        // Unity = 0 dB => normalized ≈ 0.909
        assert!((n - 60.0 / 66.0).abs() < 0.001);
    }

    #[test]
    fn full_pipeline_silence() {
        let db = LevelMeter::linear_to_db(0.0);
        let n = LevelMeter::db_to_normalized(db);
        assert!((n - 0.0).abs() < 0.001);
    }

    #[test]
    fn full_pipeline_clipping_signal() {
        let db = LevelMeter::linear_to_db(2.0); // ~+6 dB
        let n = LevelMeter::db_to_normalized(db);
        assert!((n - 1.0).abs() < 0.001);
    }
}
