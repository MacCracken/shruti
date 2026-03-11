/// All user-triggerable actions in the DAW.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Transport
    Play,
    Stop,
    Pause,
    Record,
    ToggleLoop,
    Rewind,
    FastForward,

    // Editing
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
    SplitAtPlayhead,
    Duplicate,

    // View
    ToggleArrangement,
    ToggleMixer,
    ToggleBrowser,
    ZoomIn,
    ZoomOut,
    ZoomToFit,

    // Tracks
    NewAudioTrack,
    NewBusTrack,
    ToggleMute,
    ToggleSolo,
    ToggleArm,

    // File
    NewSession,
    OpenSession,
    SaveSession,
    ExportAudio,

    // Navigation
    GoToStart,
    GoToEnd,
}
