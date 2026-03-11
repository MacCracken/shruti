use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals, epaint::Shadow};

use super::colors::ThemeColors;

/// Apply the DAW theme to an egui context.
pub fn apply_theme(ctx: &egui::Context, colors: &ThemeColors) {
    let mut visuals = Visuals::dark();

    // Window and panel backgrounds
    visuals.panel_fill = colors.bg_secondary();
    visuals.window_fill = colors.bg_secondary();
    visuals.extreme_bg_color = colors.bg_primary();
    visuals.faint_bg_color = colors.bg_tertiary();
    visuals.code_bg_color = colors.surface();

    // Text colors
    visuals.override_text_color = Some(colors.text_primary());

    // Selection
    visuals.selection.bg_fill = colors.accent().linear_multiply(0.4);
    visuals.selection.stroke = Stroke::new(1.0, colors.accent());

    // Hyperlinks
    visuals.hyperlink_color = colors.accent();

    // Shadows
    visuals.window_shadow = Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(80),
    };
    visuals.popup_shadow = Shadow {
        offset: [0, 2],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(60),
    };

    // Widget styling
    let rounding = CornerRadius::same(4);

    visuals.widgets.noninteractive.bg_fill = colors.surface();
    visuals.widgets.noninteractive.weak_bg_fill = colors.bg_tertiary();
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, colors.separator());
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors.text_secondary());
    visuals.widgets.noninteractive.corner_radius = rounding;

    visuals.widgets.inactive.bg_fill = colors.surface();
    visuals.widgets.inactive.weak_bg_fill = colors.surface();
    visuals.widgets.inactive.bg_stroke = Stroke::new(0.5, colors.separator());
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors.text_primary());
    visuals.widgets.inactive.corner_radius = rounding;

    visuals.widgets.hovered.bg_fill = colors.surface_hover();
    visuals.widgets.hovered.weak_bg_fill = colors.surface_hover();
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, colors.accent());
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, colors.text_primary());
    visuals.widgets.hovered.corner_radius = rounding;

    visuals.widgets.active.bg_fill = colors.accent();
    visuals.widgets.active.weak_bg_fill = colors.accent();
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, colors.accent_hover());
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.corner_radius = rounding;

    visuals.widgets.open.bg_fill = colors.surface_hover();
    visuals.widgets.open.weak_bg_fill = colors.surface_hover();
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, colors.accent());
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, colors.text_primary());
    visuals.widgets.open.corner_radius = rounding;

    ctx.set_visuals(visuals);

    // Font sizes — compact DAW-style
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new(10.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(12.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(14.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(11.0, FontFamily::Monospace),
        ),
    ]
    .into();

    style.spacing.item_spacing = [4.0, 3.0].into();
    style.spacing.button_padding = [6.0, 2.0].into();
    style.spacing.indent = 16.0;

    ctx.set_style(style);
}
