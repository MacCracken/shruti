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
