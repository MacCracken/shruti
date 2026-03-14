//! Project and session management with serialization.

#![deny(unsafe_code)]

pub mod audio_pool;
pub mod automation;
pub mod edit;
pub mod error;
pub mod midi;
pub mod preferences;
pub mod region;
pub mod session;
pub mod store;
pub mod timeline;
pub mod track;
pub mod transport;
pub mod types;
pub mod undo;

pub use automation::{AutomationLane, AutomationPoint, AutomationTarget, CurveType};
pub use edit::EditCommand;
pub use error::SessionError;
pub use midi::{ControlChange, MidiClip, NoteEvent};
pub use preferences::{Preferences, RecordingConfig};
pub use region::{Region, RegionId};
pub use session::Session;
pub use timeline::Timeline;
pub use track::{
    OutputRouting, Send, SendPosition, Track, TrackGroup, TrackGroupId, TrackId, TrackKind,
    TrackTemplate,
};
pub use transport::{Transport, TransportState};
pub use types::{FramePos, TrackSlot};
pub use undo::UndoManager;
