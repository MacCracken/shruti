use shruti_session::{RegionId, Session};

use crate::views::browser::BrowserTab;

/// Represents a scanned plugin in the browser.
#[derive(Debug, Clone)]
pub struct PluginEntry {
    pub name: String,
    pub format: String,
    pub path: String,
}

/// Which main view is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Arrangement,
    Mixer,
}

/// All mutable UI state bridging the session/engine to the views.
pub struct UiState {
    /// The current session being edited.
    pub session: Session,

    // View state
    pub view_mode: ViewMode,
    pub show_browser: bool,
    pub browser_tab: BrowserTab,

    // Arrangement state
    pub scroll_x: f64,
    pub pixels_per_frame: f64,
    pub selected_track: Option<usize>,
    pub selected_region: Option<RegionId>,

    // Transport
    pub recording: bool,

    // Meter data (from engine): (peak_lr, rms_lr) per track
    pub meter_levels: Vec<([f32; 2], [f32; 2])>,

    // Browser state
    pub file_entries: Vec<String>,
    pub plugin_entries: Vec<PluginEntry>,
    pub plugin_search: String,

    /// Whether the theme has been applied.
    pub theme_applied: bool,
}

impl UiState {
    pub fn new(session: Session) -> Self {
        let track_count = session.tracks.len();
        Self {
            session,
            view_mode: ViewMode::Arrangement,
            show_browser: true,
            browser_tab: BrowserTab::Files,
            scroll_x: 0.0,
            pixels_per_frame: 0.01, // ~480 pixels per second at 48kHz
            selected_track: None,
            selected_region: None,
            recording: false,
            meter_levels: vec![([0.0; 2], [0.0; 2]); track_count],
            file_entries: Vec::new(),
            plugin_entries: Vec::new(),
            plugin_search: String::new(),
            theme_applied: false,
        }
    }
}
