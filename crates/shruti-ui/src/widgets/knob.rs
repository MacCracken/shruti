use std::f32::consts::PI;

use egui::{Color32, Response, Sense, Stroke, Ui, Widget, pos2, vec2};

use crate::theme::ThemeColors;

/// Rotary knob widget for pan and other continuous parameters.
pub struct Knob<'a> {
    value: &'a mut f32,
    min: f32,
    max: f32,
    colors: &'a ThemeColors,
    radius: f32,
    label: &'a str,
}

impl<'a> Knob<'a> {
    pub fn new(value: &'a mut f32, min: f32, max: f32, colors: &'a ThemeColors) -> Self {
        Self {
            value,
            min,
            max,
            colors,
            radius: 14.0,
            label: "",
        }
    }

    /// Pan knob: -1.0 to 1.0, center default.
    pub fn pan(value: &'a mut f32, colors: &'a ThemeColors) -> Self {
        Self::new(value, -1.0, 1.0, colors).with_label("Pan")
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }

    fn normalized(&self) -> f32 {
        (*self.value - self.min) / (self.max - self.min)
    }

    fn set_from_normalized(&mut self, n: f32) {
        *self.value = (n * (self.max - self.min) + self.min).clamp(self.min, self.max);
    }
}

impl Widget for Knob<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let total_height = self.radius * 2.0 + if self.label.is_empty() { 0.0 } else { 12.0 };
        let desired_size = vec2(self.radius * 2.0, total_height);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        // Handle drag
        if response.dragged() {
            let delta = -response.drag_delta().y / 100.0;
            let new_n = (self.normalized() + delta).clamp(0.0, 1.0);
            self.set_from_normalized(new_n);
        }

        // Double-click to reset to center
        if response.double_clicked() {
            *self.value = (self.min + self.max) / 2.0;
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            let center = pos2(rect.center().x, rect.top() + self.radius);

            // Arc parameters: 135° to 405° (270° sweep)
            let start_angle = 0.75 * PI;
            let sweep = 1.5 * PI;
            let r = self.radius - 2.0;

            // Background arc
            let segments = 32;
            for i in 0..segments {
                let a1 = start_angle + sweep * (i as f32 / segments as f32);
                let a2 = start_angle + sweep * ((i + 1) as f32 / segments as f32);
                painter.line_segment(
                    [
                        pos2(center.x + r * a1.cos(), center.y + r * a1.sin()),
                        pos2(center.x + r * a2.cos(), center.y + r * a2.sin()),
                    ],
                    Stroke::new(3.0, self.colors.bg_primary()),
                );
            }

            // Value arc
            let value_angle = start_angle + sweep * self.normalized();
            let value_segments = (segments as f32 * self.normalized()) as usize;
            let arc_color = if response.hovered() || response.dragged() {
                self.colors.accent_hover()
            } else {
                self.colors.accent()
            };

            for i in 0..value_segments {
                let a1 = start_angle + sweep * (i as f32 / segments as f32);
                let a2 = start_angle + sweep * ((i + 1) as f32 / segments as f32);
                painter.line_segment(
                    [
                        pos2(center.x + r * a1.cos(), center.y + r * a1.sin()),
                        pos2(center.x + r * a2.cos(), center.y + r * a2.sin()),
                    ],
                    Stroke::new(3.0, arc_color),
                );
            }

            // Indicator dot
            let dot_r = r - 4.0;
            let dot_pos = pos2(
                center.x + dot_r * value_angle.cos(),
                center.y + dot_r * value_angle.sin(),
            );
            painter.circle_filled(dot_pos, 2.5, Color32::WHITE);

            // Center circle
            painter.circle_filled(center, 4.0, self.colors.surface());

            // Label
            if !self.label.is_empty() {
                let label_pos = pos2(center.x, rect.bottom() - 2.0);
                painter.text(
                    label_pos,
                    egui::Align2::CENTER_BOTTOM,
                    self.label,
                    egui::FontId::new(9.0, egui::FontFamily::Proportional),
                    self.colors.text_secondary(),
                );
            }
        }

        let display_val = if self.min == -1.0 && self.max == 1.0 {
            if (*self.value).abs() < 0.01 {
                "C".to_string()
            } else if *self.value < 0.0 {
                format!("L{:.0}", (-*self.value * 100.0))
            } else {
                format!("R{:.0}", (*self.value * 100.0))
            }
        } else {
            format!("{:.2}", *self.value)
        };

        response.on_hover_text(display_val)
    }
}

#[cfg(test)]
fn format_knob_display(value: f32, min: f32, max: f32) -> String {
    if min == -1.0 && max == 1.0 {
        if value.abs() < 0.01 {
            "C".to_string()
        } else if value < 0.0 {
            format!("L{:.0}", (-value * 100.0))
        } else {
            format!("R{:.0}", (value * 100.0))
        }
    } else {
        format!("{value:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeColors;

    fn colors() -> ThemeColors {
        ThemeColors::default()
    }

    // --- normalized tests ---

    #[test]
    fn normalized_at_min_is_zero() {
        let mut val = -1.0;
        let colors = colors();
        let knob = Knob::new(&mut val, -1.0, 1.0, &colors);
        assert!((knob.normalized() - 0.0).abs() < 0.001);
    }

    #[test]
    fn normalized_at_max_is_one() {
        let mut val = 1.0;
        let colors = colors();
        let knob = Knob::new(&mut val, -1.0, 1.0, &colors);
        assert!((knob.normalized() - 1.0).abs() < 0.001);
    }

    #[test]
    fn normalized_at_center() {
        let mut val = 0.0;
        let colors = colors();
        let knob = Knob::new(&mut val, -1.0, 1.0, &colors);
        assert!((knob.normalized() - 0.5).abs() < 0.001);
    }

    #[test]
    fn normalized_custom_range() {
        let mut val = 50.0;
        let colors = colors();
        let knob = Knob::new(&mut val, 0.0, 100.0, &colors);
        assert!((knob.normalized() - 0.5).abs() < 0.001);
    }

    #[test]
    fn normalized_quarter() {
        let mut val = 25.0;
        let colors = colors();
        let knob = Knob::new(&mut val, 0.0, 100.0, &colors);
        assert!((knob.normalized() - 0.25).abs() < 0.001);
    }

    // --- set_from_normalized tests ---

    #[test]
    fn set_from_normalized_zero_gives_min() {
        let mut val = 0.5;
        let colors = colors();
        let mut knob = Knob::new(&mut val, -1.0, 1.0, &colors);
        knob.set_from_normalized(0.0);
        assert!((*knob.value - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn set_from_normalized_one_gives_max() {
        let mut val = 0.0;
        let colors = colors();
        let mut knob = Knob::new(&mut val, -1.0, 1.0, &colors);
        knob.set_from_normalized(1.0);
        assert!((*knob.value - 1.0).abs() < 0.001);
    }

    #[test]
    fn set_from_normalized_half_gives_center() {
        let mut val = -1.0;
        let colors = colors();
        let mut knob = Knob::new(&mut val, -1.0, 1.0, &colors);
        knob.set_from_normalized(0.5);
        assert!((*knob.value - 0.0).abs() < 0.001);
    }

    #[test]
    fn set_from_normalized_clamps_above_one() {
        let mut val = 0.0;
        let colors = colors();
        let mut knob = Knob::new(&mut val, 0.0, 100.0, &colors);
        knob.set_from_normalized(1.5);
        assert!((*knob.value - 100.0).abs() < 0.001);
    }

    #[test]
    fn set_from_normalized_clamps_below_zero() {
        let mut val = 50.0;
        let colors = colors();
        let mut knob = Knob::new(&mut val, 0.0, 100.0, &colors);
        knob.set_from_normalized(-0.5);
        assert!((*knob.value - 0.0).abs() < 0.001);
    }

    #[test]
    fn set_from_normalized_roundtrip() {
        let mut val = 0.0;
        let colors = colors();
        for &n in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            let mut knob = Knob::new(&mut val, -1.0, 1.0, &colors);
            knob.set_from_normalized(n);
            let result = knob.normalized();
            assert!(
                (result - n).abs() < 0.001,
                "roundtrip failed for n={n}: got {result}"
            );
        }
    }

    // --- pan display formatting ---

    #[test]
    fn pan_display_center() {
        assert_eq!(format_knob_display(0.0, -1.0, 1.0), "C");
    }

    #[test]
    fn pan_display_near_center() {
        assert_eq!(format_knob_display(0.005, -1.0, 1.0), "C");
        assert_eq!(format_knob_display(-0.005, -1.0, 1.0), "C");
    }

    #[test]
    fn pan_display_full_left() {
        assert_eq!(format_knob_display(-1.0, -1.0, 1.0), "L100");
    }

    #[test]
    fn pan_display_full_right() {
        assert_eq!(format_knob_display(1.0, -1.0, 1.0), "R100");
    }

    #[test]
    fn pan_display_half_left() {
        assert_eq!(format_knob_display(-0.5, -1.0, 1.0), "L50");
    }

    #[test]
    fn pan_display_half_right() {
        assert_eq!(format_knob_display(0.5, -1.0, 1.0), "R50");
    }

    #[test]
    fn non_pan_display_format() {
        assert_eq!(format_knob_display(0.75, 0.0, 1.0), "0.75");
        assert_eq!(format_knob_display(50.0, 0.0, 100.0), "50.00");
    }

    // --- builder tests ---

    #[test]
    fn pan_constructor() {
        let mut val = 0.0;
        let colors = colors();
        let knob = Knob::pan(&mut val, &colors);
        assert_eq!(knob.min, -1.0);
        assert_eq!(knob.max, 1.0);
        assert_eq!(knob.label, "Pan");
    }

    #[test]
    fn with_label() {
        let mut val = 0.0;
        let colors = colors();
        let knob = Knob::new(&mut val, 0.0, 1.0, &colors).with_label("Gain");
        assert_eq!(knob.label, "Gain");
    }

    #[test]
    fn default_radius() {
        let mut val = 0.0;
        let colors = colors();
        let knob = Knob::new(&mut val, 0.0, 1.0, &colors);
        assert_eq!(knob.radius, 14.0);
    }

    #[test]
    fn default_label_is_empty() {
        let mut val = 0.0;
        let colors = colors();
        let knob = Knob::new(&mut val, 0.0, 1.0, &colors);
        assert_eq!(knob.label, "");
    }
}
