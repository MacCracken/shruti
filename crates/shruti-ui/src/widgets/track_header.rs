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
pub struct TrackHeaderResponse {
    pub mute_clicked: bool,
    pub solo_clicked: bool,
    pub arm_clicked: bool,
    pub selected: bool,
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
