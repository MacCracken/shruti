use std::collections::HashSet;

use shruti_session::undo::UndoManager;
use shruti_session::{Region, RegionId, Session, TrackGroupId};

use crate::views::browser::BrowserTab;

/// Describes an active drag operation in the arrangement view.
#[derive(Debug, Clone)]
pub enum ArrangementDrag {
    /// Dragging a region to a new timeline position (and possibly a different track).
    MoveRegion {
        region_id: RegionId,
        track_index: usize,
        start_frame: u64,
        /// X offset from region left edge to mouse position at drag start.
        grab_offset_px: f32,
    },
    /// Resizing a region from the left edge (trim start).
    TrimStart {
        region_id: RegionId,
        track_index: usize,
        original_pos: u64,
        original_offset: u64,
        original_duration: u64,
    },
    /// Resizing a region from the right edge (trim end).
    TrimEnd {
        region_id: RegionId,
        track_index: usize,
        original_duration: u64,
    },
    /// Dragging a track header to reorder.
    ReorderTrack {
        from_index: usize,
        current_index: usize,
    },
}

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
    Settings,
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
    /// Active drag operation in the arrangement view.
    pub drag: Option<ArrangementDrag>,

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

    /// Undo/redo manager.
    pub undo: UndoManager,

    /// Clipboard for cut/copy/paste of regions.
    pub clipboard_region: Option<Region>,

    /// Set of collapsed track group IDs (mirrors group.collapsed for fast lookup).
    pub collapsed_groups: HashSet<TrackGroupId>,
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
            drag: None,
            recording: false,
            meter_levels: vec![([0.0; 2], [0.0; 2]); track_count],
            file_entries: Vec::new(),
            plugin_entries: Vec::new(),
            plugin_search: String::new(),
            theme_applied: false,
            undo: UndoManager::default(),
            clipboard_region: None,
            collapsed_groups: HashSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> Session {
        Session::new("Test Session", 48000, 512)
    }

    #[test]
    fn new_state_has_arrangement_view() {
        let state = UiState::new(make_session());
        assert_eq!(state.view_mode, ViewMode::Arrangement);
    }

    #[test]
    fn new_state_shows_browser() {
        let state = UiState::new(make_session());
        assert!(state.show_browser);
    }

    #[test]
    fn new_state_has_files_browser_tab() {
        let state = UiState::new(make_session());
        assert_eq!(state.browser_tab, BrowserTab::Files);
    }

    #[test]
    fn new_state_scroll_and_zoom_defaults() {
        let state = UiState::new(make_session());
        assert_eq!(state.scroll_x, 0.0);
        assert!((state.pixels_per_frame - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn new_state_no_selection() {
        let state = UiState::new(make_session());
        assert!(state.selected_track.is_none());
        assert!(state.selected_region.is_none());
    }

    #[test]
    fn new_state_not_recording() {
        let state = UiState::new(make_session());
        assert!(!state.recording);
    }

    #[test]
    fn new_state_meter_levels_match_track_count() {
        let session = make_session();
        let track_count = session.tracks.len();
        let state = UiState::new(session);
        assert_eq!(state.meter_levels.len(), track_count);
        for (peak, rms) in &state.meter_levels {
            assert_eq!(*peak, [0.0, 0.0]);
            assert_eq!(*rms, [0.0, 0.0]);
        }
    }

    #[test]
    fn new_state_empty_browser_lists() {
        let state = UiState::new(make_session());
        assert!(state.file_entries.is_empty());
        assert!(state.plugin_entries.is_empty());
        assert!(state.plugin_search.is_empty());
    }

    #[test]
    fn new_state_theme_not_applied() {
        let state = UiState::new(make_session());
        assert!(!state.theme_applied);
    }

    #[test]
    fn view_mode_equality() {
        assert_eq!(ViewMode::Arrangement, ViewMode::Arrangement);
        assert_eq!(ViewMode::Mixer, ViewMode::Mixer);
        assert_ne!(ViewMode::Arrangement, ViewMode::Mixer);
    }

    #[test]
    fn test_new_state_no_drag() {
        let state = UiState::new(make_session());
        assert!(state.drag.is_none());
    }

    #[test]
    fn test_arrangement_drag_move_region() {
        let region_id = RegionId::new();
        let drag = ArrangementDrag::MoveRegion {
            region_id,
            track_index: 2,
            start_frame: 48000,
            grab_offset_px: 15.5,
        };
        match &drag {
            ArrangementDrag::MoveRegion {
                region_id: rid,
                track_index,
                start_frame,
                grab_offset_px,
            } => {
                assert_eq!(*rid, region_id);
                assert_eq!(*track_index, 2);
                assert_eq!(*start_frame, 48000);
                assert!((*grab_offset_px - 15.5).abs() < f32::EPSILON);
            }
            _ => panic!("expected MoveRegion variant"),
        }
    }

    #[test]
    fn test_arrangement_drag_trim_start() {
        let region_id = RegionId::new();
        let drag = ArrangementDrag::TrimStart {
            region_id,
            track_index: 1,
            original_pos: 1000,
            original_offset: 200,
            original_duration: 5000,
        };
        match &drag {
            ArrangementDrag::TrimStart {
                region_id: rid,
                track_index,
                original_pos,
                original_offset,
                original_duration,
            } => {
                assert_eq!(*rid, region_id);
                assert_eq!(*track_index, 1);
                assert_eq!(*original_pos, 1000);
                assert_eq!(*original_offset, 200);
                assert_eq!(*original_duration, 5000);
            }
            _ => panic!("expected TrimStart variant"),
        }
    }

    #[test]
    fn test_arrangement_drag_trim_end() {
        let region_id = RegionId::new();
        let drag = ArrangementDrag::TrimEnd {
            region_id,
            track_index: 3,
            original_duration: 9600,
        };
        match &drag {
            ArrangementDrag::TrimEnd {
                region_id: rid,
                track_index,
                original_duration,
            } => {
                assert_eq!(*rid, region_id);
                assert_eq!(*track_index, 3);
                assert_eq!(*original_duration, 9600);
            }
            _ => panic!("expected TrimEnd variant"),
        }
    }

    #[test]
    fn test_arrangement_drag_reorder_track() {
        let drag = ArrangementDrag::ReorderTrack {
            from_index: 0,
            current_index: 2,
        };
        match &drag {
            ArrangementDrag::ReorderTrack {
                from_index,
                current_index,
            } => {
                assert_eq!(*from_index, 0);
                assert_eq!(*current_index, 2);
            }
            _ => panic!("expected ReorderTrack variant"),
        }
    }
}
