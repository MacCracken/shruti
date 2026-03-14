use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use shruti_session::undo::UndoManager;
use shruti_session::{FramePos, Region, RegionId, Session, TrackGroupId};

use crate::views::browser::BrowserTab;
use crate::widgets::waveform::WaveformPeaks;

/// Describes an active drag operation in the arrangement view.
#[derive(Debug, Clone)]
pub enum ArrangementDrag {
    /// Dragging a region to a new timeline position (and possibly a different track).
    MoveRegion {
        region_id: RegionId,
        track_index: usize,
        start_frame: FramePos,
        /// X offset from region left edge to mouse position at drag start.
        grab_offset_px: f32,
    },
    /// Resizing a region from the left edge (trim start).
    TrimStart {
        region_id: RegionId,
        track_index: usize,
        original_pos: FramePos,
        original_offset: FramePos,
        original_duration: FramePos,
    },
    /// Resizing a region from the right edge (trim end).
    TrimEnd {
        region_id: RegionId,
        track_index: usize,
        original_duration: FramePos,
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
    InstrumentEditor,
    PianoRoll,
}

// ---------------------------------------------------------------------------
// Toast notification system
// ---------------------------------------------------------------------------

/// Severity level for toast notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastSeverity {
    Info,
    Warning,
    Error,
}

/// A toast notification displayed as an overlay.
#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub severity: ToastSeverity,
    pub created: Instant,
    pub duration: Duration,
}

impl Toast {
    pub fn new(message: impl Into<String>, severity: ToastSeverity) -> Self {
        let duration = match severity {
            ToastSeverity::Info => Duration::from_secs(3),
            ToastSeverity::Warning => Duration::from_secs(5),
            ToastSeverity::Error => Duration::from_secs(8),
        };
        Self {
            message: message.into(),
            severity,
            created: Instant::now(),
            duration,
        }
    }

    /// Whether this toast has expired and should be removed.
    pub fn is_expired(&self) -> bool {
        self.created.elapsed() >= self.duration
    }

    /// Progress from 0.0 (just created) to 1.0 (about to expire).
    pub fn progress(&self) -> f32 {
        let elapsed = self.created.elapsed().as_secs_f32();
        (elapsed / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }
}

/// Manages a list of toasts, removing expired ones.
pub fn gc_toasts(toasts: &mut Vec<Toast>) {
    toasts.retain(|t| !t.is_expired());
}

// ---------------------------------------------------------------------------
// Background task system
// ---------------------------------------------------------------------------

/// The kind of background task currently running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundTaskKind {
    Save,
    Load,
    Export,
}

/// State of a background file I/O task.
#[derive(Debug, Clone)]
pub struct BackgroundTaskState {
    pub kind: BackgroundTaskKind,
    pub description: String,
    pub started: Instant,
}

/// Result delivered from a background task thread.
pub enum BackgroundTaskResult {
    SaveComplete(Result<(), String>),
    LoadComplete(Result<Box<Session>, String>),
    ExportComplete(Result<(), String>),
}

// ---------------------------------------------------------------------------
// Save prompt / pending action system
// ---------------------------------------------------------------------------

/// An action deferred until the user responds to a "save changes?" dialog.
#[derive(Debug, Clone)]
pub enum DeferredAction {
    NewSession,
    OpenSession,
}

// ---------------------------------------------------------------------------
// Auto-save helpers (pure logic, no egui dependency)
// ---------------------------------------------------------------------------

/// Compute the backup file path for a session file.
/// Given `/foo/bar/project.shruti`, returns `/foo/bar/.project.shruti_backup`.
pub fn backup_path_for(session_path: &Path) -> PathBuf {
    let file_name = session_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "session".to_string());
    let backup_name = format!(".{file_name}_backup");
    session_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(backup_name)
}

/// Returns true if a backup file exists for the given session path.
pub fn has_backup(session_path: &Path) -> bool {
    backup_path_for(session_path).exists()
}

/// Format a title bar string including dirty indicator.
pub fn title_with_dirty(session_name: &str, dirty: bool) -> String {
    if dirty {
        format!("*{session_name}")
    } else {
        session_name.to_string()
    }
}

/// Auto-save interval.
pub const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// UI State
// ---------------------------------------------------------------------------

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

    /// Cached waveform peaks per region, recomputed only when region data changes.
    pub waveform_cache: HashMap<RegionId, WaveformPeaks>,

    /// The name of the currently applied theme (used to detect theme changes).
    pub applied_theme_name: Option<String>,

    /// Whether snap-to-grid is enabled for region dragging.
    pub snap_enabled: bool,

    /// Undo/redo manager.
    pub undo: UndoManager,

    /// Clipboard for cut/copy/paste of regions.
    pub clipboard_region: Option<Region>,

    /// Set of collapsed track group IDs (mirrors group.collapsed for fast lookup).
    pub collapsed_groups: HashSet<TrackGroupId>,

    // -- Auto-save & dirty tracking --
    /// Whether the session has unsaved changes.
    pub dirty: bool,
    /// Path where the session was last saved/opened (None = never saved).
    pub session_path: Option<PathBuf>,
    /// Last time the session was auto-saved.
    pub last_autosave: Instant,

    // -- Toast notifications --
    /// Active toast notifications.
    pub toasts: Vec<Toast>,

    // -- Background tasks --
    /// Current background task state (for progress display).
    pub background_task: Option<BackgroundTaskState>,
    /// Receiver for background task results.
    pub bg_result_rx: Option<mpsc::Receiver<BackgroundTaskResult>>,

    // -- Save prompt / deferred actions --
    /// Pending action waiting for save-prompt resolution.
    pub pending_action: Option<DeferredAction>,
    /// Whether the save-prompt dialog is currently open.
    pub show_save_prompt: bool,

    // -- Audio engine init feedback --
    /// Error from audio engine initialization (shown as dialog on startup).
    pub engine_init_error: Option<String>,
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
            waveform_cache: HashMap::new(),
            applied_theme_name: None,
            snap_enabled: true,
            undo: UndoManager::default(),
            clipboard_region: None,
            collapsed_groups: HashSet::new(),
            dirty: false,
            session_path: None,
            last_autosave: Instant::now(),
            toasts: Vec::new(),
            background_task: None,
            bg_result_rx: None,
            pending_action: None,
            show_save_prompt: false,
            engine_init_error: None,
        }
    }

    /// Mark the session as dirty (has unsaved changes).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark the session as clean (just saved).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Add a toast notification.
    pub fn push_toast(&mut self, message: impl Into<String>, severity: ToastSeverity) {
        self.toasts.push(Toast::new(message, severity));
    }

    /// Check if auto-save should fire and reset the timer if so.
    /// Returns true if an auto-save should be triggered.
    pub fn should_autosave(&mut self) -> bool {
        if self.dirty
            && self.session_path.is_some()
            && self.background_task.is_none()
            && self.last_autosave.elapsed() >= AUTOSAVE_INTERVAL
        {
            self.last_autosave = Instant::now();
            return true;
        }
        false
    }

    /// Title string for the window title bar.
    pub fn title_bar_text(&self) -> String {
        title_with_dirty(&self.session.name, self.dirty)
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
    fn new_state_waveform_cache_empty() {
        let state = UiState::new(make_session());
        assert!(state.waveform_cache.is_empty());
    }

    #[test]
    fn new_state_no_applied_theme_name() {
        let state = UiState::new(make_session());
        assert!(state.applied_theme_name.is_none());
    }

    #[test]
    fn new_state_snap_enabled() {
        let state = UiState::new(make_session());
        assert!(state.snap_enabled);
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
            start_frame: FramePos(48000),
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
                assert_eq!(*start_frame, FramePos(48000));
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
            original_pos: FramePos(1000),
            original_offset: FramePos(200),
            original_duration: FramePos(5000),
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
                assert_eq!(*original_pos, FramePos(1000));
                assert_eq!(*original_offset, FramePos(200));
                assert_eq!(*original_duration, FramePos(5000));
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
            original_duration: FramePos(9600),
        };
        match &drag {
            ArrangementDrag::TrimEnd {
                region_id: rid,
                track_index,
                original_duration,
            } => {
                assert_eq!(*rid, region_id);
                assert_eq!(*track_index, 3);
                assert_eq!(*original_duration, FramePos(9600));
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

    // ---------------------------------------------------------------
    // Toast notification tests
    // ---------------------------------------------------------------

    #[test]
    fn toast_info_default_duration() {
        let toast = Toast::new("hello", ToastSeverity::Info);
        assert_eq!(toast.duration, Duration::from_secs(3));
        assert_eq!(toast.severity, ToastSeverity::Info);
        assert_eq!(toast.message, "hello");
    }

    #[test]
    fn toast_warning_default_duration() {
        let toast = Toast::new("warn", ToastSeverity::Warning);
        assert_eq!(toast.duration, Duration::from_secs(5));
    }

    #[test]
    fn toast_error_default_duration() {
        let toast = Toast::new("err", ToastSeverity::Error);
        assert_eq!(toast.duration, Duration::from_secs(8));
    }

    #[test]
    fn toast_not_expired_immediately() {
        let toast = Toast::new("msg", ToastSeverity::Info);
        assert!(!toast.is_expired());
    }

    #[test]
    fn toast_progress_starts_near_zero() {
        let toast = Toast::new("msg", ToastSeverity::Info);
        assert!(toast.progress() < 0.1);
    }

    #[test]
    fn gc_toasts_removes_expired() {
        let mut toasts = vec![
            Toast {
                message: "old".into(),
                severity: ToastSeverity::Info,
                created: Instant::now() - Duration::from_secs(100),
                duration: Duration::from_secs(3),
            },
            Toast::new("new", ToastSeverity::Info),
        ];
        gc_toasts(&mut toasts);
        assert_eq!(toasts.len(), 1);
        assert_eq!(toasts[0].message, "new");
    }

    #[test]
    fn gc_toasts_keeps_all_when_none_expired() {
        let mut toasts = vec![
            Toast::new("a", ToastSeverity::Info),
            Toast::new("b", ToastSeverity::Warning),
        ];
        gc_toasts(&mut toasts);
        assert_eq!(toasts.len(), 2);
    }

    // ---------------------------------------------------------------
    // Auto-save / backup path tests
    // ---------------------------------------------------------------

    #[test]
    fn backup_path_for_normal_file() {
        let p = PathBuf::from("/home/user/projects/song.shruti");
        let bp = backup_path_for(&p);
        assert_eq!(bp, PathBuf::from("/home/user/projects/.song.shruti_backup"));
    }

    #[test]
    fn backup_path_for_no_parent() {
        let p = PathBuf::from("session.shruti");
        let bp = backup_path_for(&p);
        assert_eq!(bp, PathBuf::from(".session.shruti_backup"));
    }

    #[test]
    fn title_with_dirty_shows_asterisk() {
        assert_eq!(title_with_dirty("My Song", true), "*My Song");
        assert_eq!(title_with_dirty("My Song", false), "My Song");
    }

    // ---------------------------------------------------------------
    // Dirty tracking tests
    // ---------------------------------------------------------------

    #[test]
    fn new_state_not_dirty() {
        let state = UiState::new(make_session());
        assert!(!state.dirty);
    }

    #[test]
    fn mark_dirty_sets_dirty() {
        let mut state = UiState::new(make_session());
        state.mark_dirty();
        assert!(state.dirty);
    }

    #[test]
    fn mark_clean_clears_dirty() {
        let mut state = UiState::new(make_session());
        state.mark_dirty();
        state.mark_clean();
        assert!(!state.dirty);
    }

    #[test]
    fn title_bar_text_reflects_dirty() {
        let mut state = UiState::new(make_session());
        assert_eq!(state.title_bar_text(), "Test Session");
        state.mark_dirty();
        assert_eq!(state.title_bar_text(), "*Test Session");
    }

    #[test]
    fn push_toast_adds_to_list() {
        let mut state = UiState::new(make_session());
        state.push_toast("test", ToastSeverity::Error);
        assert_eq!(state.toasts.len(), 1);
        assert_eq!(state.toasts[0].message, "test");
    }

    #[test]
    fn should_autosave_false_when_clean() {
        let mut state = UiState::new(make_session());
        state.session_path = Some(PathBuf::from("/tmp/test.shruti"));
        state.last_autosave = Instant::now() - Duration::from_secs(120);
        assert!(!state.should_autosave()); // not dirty
    }

    #[test]
    fn should_autosave_false_when_no_path() {
        let mut state = UiState::new(make_session());
        state.dirty = true;
        state.last_autosave = Instant::now() - Duration::from_secs(120);
        assert!(!state.should_autosave()); // no path
    }

    #[test]
    fn should_autosave_false_when_recent() {
        let mut state = UiState::new(make_session());
        state.dirty = true;
        state.session_path = Some(PathBuf::from("/tmp/test.shruti"));
        state.last_autosave = Instant::now(); // just saved
        assert!(!state.should_autosave());
    }

    #[test]
    fn should_autosave_true_when_due() {
        let mut state = UiState::new(make_session());
        state.dirty = true;
        state.session_path = Some(PathBuf::from("/tmp/test.shruti"));
        state.last_autosave = Instant::now() - Duration::from_secs(120);
        assert!(state.should_autosave());
    }

    #[test]
    fn should_autosave_resets_timer() {
        let mut state = UiState::new(make_session());
        state.dirty = true;
        state.session_path = Some(PathBuf::from("/tmp/test.shruti"));
        state.last_autosave = Instant::now() - Duration::from_secs(120);
        assert!(state.should_autosave());
        // Timer was reset, so second call should return false
        assert!(!state.should_autosave());
    }

    // ---------------------------------------------------------------
    // Background task state tests
    // ---------------------------------------------------------------

    #[test]
    fn background_task_state_construction() {
        let task = BackgroundTaskState {
            kind: BackgroundTaskKind::Save,
            description: "Saving...".into(),
            started: Instant::now(),
        };
        assert_eq!(task.kind, BackgroundTaskKind::Save);
        assert_eq!(task.description, "Saving...");
    }

    // ---------------------------------------------------------------
    // Deferred action / save prompt tests
    // ---------------------------------------------------------------

    #[test]
    fn new_state_no_pending_action() {
        let state = UiState::new(make_session());
        assert!(state.pending_action.is_none());
        assert!(!state.show_save_prompt);
    }

    #[test]
    fn new_state_no_engine_init_error() {
        let state = UiState::new(make_session());
        assert!(state.engine_init_error.is_none());
    }
}
