use egui::{CornerRadius, Ui, vec2};

use crate::theme::ThemeColors;

/// A plugin slot in a channel strip or track inspector.
pub struct PluginSlotState {
    pub name: String,
    pub bypassed: bool,
}

pub struct PluginSlotResponse {
    pub clicked: bool,
    pub bypass_toggled: bool,
}

/// Draw a plugin slot widget.
pub fn plugin_slot_ui(
    ui: &mut Ui,
    state: &PluginSlotState,
    colors: &ThemeColors,
) -> PluginSlotResponse {
    let mut response = PluginSlotResponse {
        clicked: false,
        bypass_toggled: false,
    };

    ui.horizontal(|ui| {
        // Bypass indicator
        let bypass_color = if state.bypassed {
            colors.mute_orange()
        } else {
            colors.transport_active()
        };
        let bypass_btn = ui.add(
            egui::Button::new(
                egui::RichText::new(if state.bypassed { "○" } else { "●" })
                    .size(10.0)
                    .color(bypass_color),
            )
            .fill(colors.surface())
            .corner_radius(CornerRadius::same(2))
            .min_size(vec2(16.0, 16.0)),
        );
        if bypass_btn.clicked() {
            response.bypass_toggled = true;
        }

        // Plugin name button
        let name_btn = ui.add(
            egui::Button::new(egui::RichText::new(&state.name).size(10.0).color(
                if state.bypassed {
                    colors.text_secondary()
                } else {
                    colors.text_primary()
                },
            ))
            .fill(colors.surface())
            .corner_radius(CornerRadius::same(2))
            .min_size(vec2(80.0, 16.0)),
        );
        if name_btn.clicked() {
            response.clicked = true;
        }
    });

    response
}
