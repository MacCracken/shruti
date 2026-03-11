//! Project and session management with serialization.

#![deny(unsafe_code)]

pub mod audio_pool;
pub mod edit;
pub mod region;
pub mod session;
pub mod store;
pub mod timeline;
pub mod track;
pub mod transport;
pub mod undo;

pub use edit::EditCommand;
pub use region::{Region, RegionId};
pub use session::Session;
pub use timeline::Timeline;
pub use track::{Track, TrackId, TrackKind};
pub use transport::{Transport, TransportState};
pub use undo::UndoManager;
