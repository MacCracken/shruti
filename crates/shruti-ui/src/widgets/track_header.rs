use egui::{Color32, CornerRadius, Layout, Ui, vec2};

use crate::theme::ThemeColors;

/// Track header state for the UI.
pub struct TrackHeaderState {
    pub name: String,
    pub muted: bool,
    pub solo: bool,
    pub armed: bool,
    pub selected: bool,
    pub color_index: usize,
}

/// Track color palette (10 colors, cycling).
const TRACK_COLORS: [Color32; 10] = [
    Color32::from_rgb(70, 130, 210),  // Blue
    Color32::from_rgb(80, 180, 100),  // Green
    Color32::from_rgb(200, 100, 60),  // Orange
    Color32::from_rgb(170, 80, 190),  // Purple
    Color32::from_rgb(200, 180, 50),  // Yellow
    Color32::from_rgb(80, 190, 190),  // Cyan
    Color32::from_rgb(210, 80, 100),  // Red
    Color32::from_rgb(130, 180, 70),  // Lime
    Color32::from_rgb(180, 130, 80),  // Tan
    Color32::from_rgb(120, 100, 200), // Indigo
];

pub fn track_color(index: usize) -> Color32 {
    TRACK_COLORS[index % TRACK_COLORS.len()]
}

/// Draw a track header, returning which buttons were clicked.
#[derive(Default)]
pub struct TrackHeaderResponse {
    pub mute_clicked: bool,
    pub solo_clicked: bool,
    pub arm_clicked: bool,
    pub selected: bool,
}

impl TrackHeaderState {
    /// Create a new track header state with defaults.
    pub fn new(name: impl Into<String>, color_index: usize) -> Self {
        Self {
            name: name.into(),
            muted: false,
            solo: false,
            armed: false,
            selected: false,
            color_index,
        }
    }
}

pub fn track_header_ui(
    ui: &mut Ui,
    state: &TrackHeaderState,
    colors: &ThemeColors,
    width: f32,
) -> TrackHeaderResponse {
    let mut response = TrackHeaderResponse {
        mute_clicked: false,
        solo_clicked: false,
        arm_clicked: false,
        selected: false,
    };

    let height = 60.0;
    let (rect, click_response) = ui.allocate_exact_size(vec2(width, height), egui::Sense::click());

    if click_response.clicked() {
        response.selected = true;
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);

        // Background
        let bg = if state.selected {
            colors.accent().linear_multiply(0.15)
        } else {
            colors.track_header()
        };
        painter.rect_filled(rect, 0.0, bg);

        // Color strip on left edge
        let strip_rect = rect.with_max_x(rect.left() + 4.0);
        painter.rect_filled(strip_rect, 0.0, track_color(state.color_index));

        // Bottom separator
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(0.5, colors.separator()),
        );
    }

    // UI inside the header
    let inner_rect = rect.shrink2(vec2(8.0, 4.0));
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));

    child_ui.with_layout(Layout::top_down(egui::Align::LEFT), |ui| {
        // Track name
        ui.label(
            egui::RichText::new(&state.name)
                .size(11.0)
                .color(colors.text_primary()),
        );

        ui.add_space(2.0);

        // M S R buttons
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            // Mute button
            let mute_color = if state.muted {
                colors.mute_orange()
            } else {
                colors.surface()
            };
            let mute_btn = ui.add(
                egui::Button::new(egui::RichText::new("M").size(10.0).color(if state.muted {
                    Color32::WHITE
                } else {
                    colors.text_secondary()
                }))
                .fill(mute_color)
                .corner_radius(CornerRadius::same(2))
                .min_size(vec2(20.0, 16.0)),
            );
            if mute_btn.clicked() {
                response.mute_clicked = true;
            }

            // Solo button
            let solo_color = if state.solo {
                colors.solo_yellow()
            } else {
                colors.surface()
            };
            let solo_btn = ui.add(
                egui::Button::new(egui::RichText::new("S").size(10.0).color(if state.solo {
                    Color32::BLACK
                } else {
                    colors.text_secondary()
                }))
                .fill(solo_color)
                .corner_radius(CornerRadius::same(2))
                .min_size(vec2(20.0, 16.0)),
            );
            if solo_btn.clicked() {
                response.solo_clicked = true;
            }

            // Record arm button
            let arm_color = if state.armed {
                colors.record_red()
            } else {
                colors.surface()
            };
            let arm_btn = ui.add(
                egui::Button::new(egui::RichText::new("R").size(10.0).color(if state.armed {
                    Color32::WHITE
                } else {
                    colors.text_secondary()
                }))
                .fill(arm_color)
                .corner_radius(CornerRadius::same(2))
                .min_size(vec2(20.0, 16.0)),
            );
            if arm_btn.clicked() {
                response.arm_clicked = true;
            }
        });
    });

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_color_returns_expected_first_colors() {
        assert_eq!(track_color(0), Color32::from_rgb(70, 130, 210)); // Blue
        assert_eq!(track_color(1), Color32::from_rgb(80, 180, 100)); // Green
        assert_eq!(track_color(2), Color32::from_rgb(200, 100, 60)); // Orange
        assert_eq!(track_color(3), Color32::from_rgb(170, 80, 190)); // Purple
        assert_eq!(track_color(4), Color32::from_rgb(200, 180, 50)); // Yellow
    }

    #[test]
    fn track_color_returns_remaining_colors() {
        assert_eq!(track_color(5), Color32::from_rgb(80, 190, 190)); // Cyan
        assert_eq!(track_color(6), Color32::from_rgb(210, 80, 100)); // Red
        assert_eq!(track_color(7), Color32::from_rgb(130, 180, 70)); // Lime
        assert_eq!(track_color(8), Color32::from_rgb(180, 130, 80)); // Tan
        assert_eq!(track_color(9), Color32::from_rgb(120, 100, 200)); // Indigo
    }

    #[test]
    fn track_color_wraps_around() {
        assert_eq!(track_color(10), track_color(0));
        assert_eq!(track_color(11), track_color(1));
        assert_eq!(track_color(20), track_color(0));
        assert_eq!(track_color(25), track_color(5));
    }

    #[test]
    fn track_color_large_indices() {
        assert_eq!(track_color(100), track_color(0));
        assert_eq!(track_color(103), track_color(3));
        assert_eq!(track_color(999), track_color(9));
    }

    #[test]
    fn track_header_state_new() {
        let state = TrackHeaderState::new("Audio 1", 3);
        assert_eq!(state.name, "Audio 1");
        assert_eq!(state.color_index, 3);
        assert!(!state.muted);
        assert!(!state.solo);
        assert!(!state.armed);
        assert!(!state.selected);
    }

    #[test]
    fn track_header_state_new_with_string() {
        let name = String::from("My Track");
        let state = TrackHeaderState::new(name, 0);
        assert_eq!(state.name, "My Track");
    }

    #[test]
    fn track_header_state_mutable_fields() {
        let mut state = TrackHeaderState::new("Track", 0);
        state.muted = true;
        state.solo = true;
        state.armed = true;
        state.selected = true;
        assert!(state.muted);
        assert!(state.solo);
        assert!(state.armed);
        assert!(state.selected);
    }

    #[test]
    fn track_header_response_default() {
        let resp = TrackHeaderResponse::default();
        assert!(!resp.mute_clicked);
        assert!(!resp.solo_clicked);
        assert!(!resp.arm_clicked);
        assert!(!resp.selected);
    }

    #[test]
    fn track_colors_palette_has_ten_entries() {
        // Verify all 10 colors are distinct
        let colors: Vec<Color32> = (0..10).map(track_color).collect();
        for i in 0..10 {
            for j in (i + 1)..10 {
                assert_ne!(
                    colors[i], colors[j],
                    "Colors at index {i} and {j} should differ"
                );
            }
        }
    }
}
