use egui::Color32;
use serde::{Deserialize, Serialize};

/// Color palette for the DAW theme.
/// Somewhere between Logic's polish and Reaper's density.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    /// Main background (darkest).
    pub bg_primary: [u8; 4],
    /// Secondary background (panels, sidebars).
    pub bg_secondary: [u8; 4],
    /// Tertiary background (track lanes, alternating rows).
    pub bg_tertiary: [u8; 4],
    /// Surface color (widgets, cards).
    pub surface: [u8; 4],
    /// Surface hover state.
    pub surface_hover: [u8; 4],
    /// Primary text.
    pub text_primary: [u8; 4],
    /// Secondary/dimmed text.
    pub text_secondary: [u8; 4],
    /// Accent color (selection, active elements).
    pub accent: [u8; 4],
    /// Accent hover.
    pub accent_hover: [u8; 4],
    /// Transport play/record active.
    pub transport_active: [u8; 4],
    /// Record arm color.
    pub record_red: [u8; 4],
    /// Solo color.
    pub solo_yellow: [u8; 4],
    /// Mute color.
    pub mute_orange: [u8; 4],
    /// Meter green (low level).
    pub meter_green: [u8; 4],
    /// Meter yellow (mid level).
    pub meter_yellow: [u8; 4],
    /// Meter red (clip).
    pub meter_red: [u8; 4],
    /// Waveform fill color.
    pub waveform: [u8; 4],
    /// Waveform outline.
    pub waveform_outline: [u8; 4],
    /// Playhead cursor.
    pub playhead: [u8; 4],
    /// Grid lines.
    pub grid: [u8; 4],
    /// Region/clip background.
    pub region_bg: [u8; 4],
    /// Region/clip selected.
    pub region_selected: [u8; 4],
    /// Automation curve.
    pub automation_curve: [u8; 4],
    /// Automation point.
    pub automation_point: [u8; 4],
    /// Separator/border lines.
    pub separator: [u8; 4],
    /// Scrollbar.
    pub scrollbar: [u8; 4],
    /// Track header background.
    pub track_header: [u8; 4],
}

impl ThemeColors {
    pub fn c(rgba: [u8; 4]) -> Color32 {
        Color32::from_rgba_premultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
    }

    pub fn bg_primary(&self) -> Color32 {
        Self::c(self.bg_primary)
    }
    pub fn bg_secondary(&self) -> Color32 {
        Self::c(self.bg_secondary)
    }
    pub fn bg_tertiary(&self) -> Color32 {
        Self::c(self.bg_tertiary)
    }
    pub fn surface(&self) -> Color32 {
        Self::c(self.surface)
    }
    pub fn surface_hover(&self) -> Color32 {
        Self::c(self.surface_hover)
    }
    pub fn text_primary(&self) -> Color32 {
        Self::c(self.text_primary)
    }
    pub fn text_secondary(&self) -> Color32 {
        Self::c(self.text_secondary)
    }
    pub fn accent(&self) -> Color32 {
        Self::c(self.accent)
    }
    pub fn accent_hover(&self) -> Color32 {
        Self::c(self.accent_hover)
    }
    pub fn transport_active(&self) -> Color32 {
        Self::c(self.transport_active)
    }
    pub fn record_red(&self) -> Color32 {
        Self::c(self.record_red)
    }
    pub fn solo_yellow(&self) -> Color32 {
        Self::c(self.solo_yellow)
    }
    pub fn mute_orange(&self) -> Color32 {
        Self::c(self.mute_orange)
    }
    pub fn meter_green(&self) -> Color32 {
        Self::c(self.meter_green)
    }
    pub fn meter_yellow(&self) -> Color32 {
        Self::c(self.meter_yellow)
    }
    pub fn meter_red(&self) -> Color32 {
        Self::c(self.meter_red)
    }
    pub fn waveform(&self) -> Color32 {
        Self::c(self.waveform)
    }
    pub fn waveform_outline(&self) -> Color32 {
        Self::c(self.waveform_outline)
    }
    pub fn playhead(&self) -> Color32 {
        Self::c(self.playhead)
    }
    pub fn grid(&self) -> Color32 {
        Self::c(self.grid)
    }
    pub fn region_bg(&self) -> Color32 {
        Self::c(self.region_bg)
    }
    pub fn region_selected(&self) -> Color32 {
        Self::c(self.region_selected)
    }
    pub fn automation_curve(&self) -> Color32 {
        Self::c(self.automation_curve)
    }
    pub fn automation_point(&self) -> Color32 {
        Self::c(self.automation_point)
    }
    pub fn separator(&self) -> Color32 {
        Self::c(self.separator)
    }
    pub fn scrollbar(&self) -> Color32 {
        Self::c(self.scrollbar)
    }
    pub fn track_header(&self) -> Color32 {
        Self::c(self.track_header)
    }
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            // Dark background palette (Logic-inspired darkness, Reaper density)
            bg_primary: [24, 24, 28, 255],
            bg_secondary: [32, 32, 38, 255],
            bg_tertiary: [38, 38, 44, 255],
            surface: [48, 48, 56, 255],
            surface_hover: [58, 58, 68, 255],

            // Text
            text_primary: [220, 220, 225, 255],
            text_secondary: [140, 140, 150, 255],

            // Accent (blue-ish, Logic-style)
            accent: [60, 130, 240, 255],
            accent_hover: [80, 150, 255, 255],

            // Transport
            transport_active: [60, 180, 80, 255],
            record_red: [220, 50, 50, 255],
            solo_yellow: [220, 190, 40, 255],
            mute_orange: [200, 120, 40, 255],

            // Meters
            meter_green: [40, 200, 80, 255],
            meter_yellow: [220, 200, 40, 255],
            meter_red: [220, 50, 50, 255],

            // Waveform
            waveform: [80, 160, 220, 180],
            waveform_outline: [100, 180, 240, 220],

            // Timeline
            playhead: [255, 255, 255, 200],
            grid: [60, 60, 68, 255],

            // Regions
            region_bg: [55, 80, 120, 200],
            region_selected: [70, 110, 170, 220],

            // Automation
            automation_curve: [180, 100, 220, 255],
            automation_point: [220, 130, 255, 255],

            // Chrome
            separator: [50, 50, 58, 255],
            scrollbar: [70, 70, 80, 150],
            track_header: [36, 36, 42, 255],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_colors_have_expected_values() {
        let colors = ThemeColors::default();
        assert_eq!(colors.bg_primary, [24, 24, 28, 255]);
        assert_eq!(colors.bg_secondary, [32, 32, 38, 255]);
        assert_eq!(colors.bg_tertiary, [38, 38, 44, 255]);
        assert_eq!(colors.surface, [48, 48, 56, 255]);
        assert_eq!(colors.surface_hover, [58, 58, 68, 255]);
        assert_eq!(colors.text_primary, [220, 220, 225, 255]);
        assert_eq!(colors.text_secondary, [140, 140, 150, 255]);
        assert_eq!(colors.accent, [60, 130, 240, 255]);
        assert_eq!(colors.accent_hover, [80, 150, 255, 255]);
        assert_eq!(colors.transport_active, [60, 180, 80, 255]);
        assert_eq!(colors.record_red, [220, 50, 50, 255]);
        assert_eq!(colors.solo_yellow, [220, 190, 40, 255]);
        assert_eq!(colors.mute_orange, [200, 120, 40, 255]);
        assert_eq!(colors.meter_green, [40, 200, 80, 255]);
        assert_eq!(colors.meter_yellow, [220, 200, 40, 255]);
        assert_eq!(colors.meter_red, [220, 50, 50, 255]);
        assert_eq!(colors.waveform, [80, 160, 220, 180]);
        assert_eq!(colors.waveform_outline, [100, 180, 240, 220]);
        assert_eq!(colors.playhead, [255, 255, 255, 200]);
        assert_eq!(colors.grid, [60, 60, 68, 255]);
        assert_eq!(colors.region_bg, [55, 80, 120, 200]);
        assert_eq!(colors.region_selected, [70, 110, 170, 220]);
        assert_eq!(colors.automation_curve, [180, 100, 220, 255]);
        assert_eq!(colors.automation_point, [220, 130, 255, 255]);
        assert_eq!(colors.separator, [50, 50, 58, 255]);
        assert_eq!(colors.scrollbar, [70, 70, 80, 150]);
        assert_eq!(colors.track_header, [36, 36, 42, 255]);
    }

    #[test]
    fn color_accessor_returns_correct_color32() {
        let colors = ThemeColors::default();

        // Test a representative subset of accessors
        assert_eq!(
            colors.bg_primary(),
            Color32::from_rgba_premultiplied(24, 24, 28, 255)
        );
        assert_eq!(
            colors.text_primary(),
            Color32::from_rgba_premultiplied(220, 220, 225, 255)
        );
        assert_eq!(
            colors.accent(),
            Color32::from_rgba_premultiplied(60, 130, 240, 255)
        );
        assert_eq!(
            colors.waveform(),
            Color32::from_rgba_premultiplied(80, 160, 220, 180)
        );
        assert_eq!(
            colors.playhead(),
            Color32::from_rgba_premultiplied(255, 255, 255, 200)
        );
        assert_eq!(
            colors.meter_green(),
            Color32::from_rgba_premultiplied(40, 200, 80, 255)
        );
        assert_eq!(
            colors.record_red(),
            Color32::from_rgba_premultiplied(220, 50, 50, 255)
        );
        assert_eq!(
            colors.separator(),
            Color32::from_rgba_premultiplied(50, 50, 58, 255)
        );
    }

    #[test]
    fn c_helper_converts_rgba_to_color32() {
        let c = ThemeColors::c([100, 200, 50, 128]);
        assert_eq!(c, Color32::from_rgba_premultiplied(100, 200, 50, 128));
    }

    #[test]
    fn all_accessors_match_their_fields() {
        let colors = ThemeColors::default();

        assert_eq!(colors.bg_primary(), ThemeColors::c(colors.bg_primary));
        assert_eq!(colors.bg_secondary(), ThemeColors::c(colors.bg_secondary));
        assert_eq!(colors.bg_tertiary(), ThemeColors::c(colors.bg_tertiary));
        assert_eq!(colors.surface(), ThemeColors::c(colors.surface));
        assert_eq!(colors.surface_hover(), ThemeColors::c(colors.surface_hover));
        assert_eq!(colors.text_primary(), ThemeColors::c(colors.text_primary));
        assert_eq!(
            colors.text_secondary(),
            ThemeColors::c(colors.text_secondary)
        );
        assert_eq!(colors.accent(), ThemeColors::c(colors.accent));
        assert_eq!(colors.accent_hover(), ThemeColors::c(colors.accent_hover));
        assert_eq!(
            colors.transport_active(),
            ThemeColors::c(colors.transport_active)
        );
        assert_eq!(colors.record_red(), ThemeColors::c(colors.record_red));
        assert_eq!(colors.solo_yellow(), ThemeColors::c(colors.solo_yellow));
        assert_eq!(colors.mute_orange(), ThemeColors::c(colors.mute_orange));
        assert_eq!(colors.meter_green(), ThemeColors::c(colors.meter_green));
        assert_eq!(colors.meter_yellow(), ThemeColors::c(colors.meter_yellow));
        assert_eq!(colors.meter_red(), ThemeColors::c(colors.meter_red));
        assert_eq!(colors.waveform(), ThemeColors::c(colors.waveform));
        assert_eq!(
            colors.waveform_outline(),
            ThemeColors::c(colors.waveform_outline)
        );
        assert_eq!(colors.playhead(), ThemeColors::c(colors.playhead));
        assert_eq!(colors.grid(), ThemeColors::c(colors.grid));
        assert_eq!(colors.region_bg(), ThemeColors::c(colors.region_bg));
        assert_eq!(
            colors.region_selected(),
            ThemeColors::c(colors.region_selected)
        );
        assert_eq!(
            colors.automation_curve(),
            ThemeColors::c(colors.automation_curve)
        );
        assert_eq!(
            colors.automation_point(),
            ThemeColors::c(colors.automation_point)
        );
        assert_eq!(colors.separator(), ThemeColors::c(colors.separator));
        assert_eq!(colors.scrollbar(), ThemeColors::c(colors.scrollbar));
        assert_eq!(colors.track_header(), ThemeColors::c(colors.track_header));
    }

    #[test]
    fn serde_roundtrip() {
        let original = ThemeColors::default();
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: ThemeColors = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.bg_primary, original.bg_primary);
        assert_eq!(deserialized.accent, original.accent);
        assert_eq!(deserialized.waveform, original.waveform);
        assert_eq!(deserialized.scrollbar, original.scrollbar);
        assert_eq!(deserialized.track_header, original.track_header);
    }
}
