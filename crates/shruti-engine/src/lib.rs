//! Real-time audio engine with lock-free graph processing.

pub mod backend;
pub mod graph;
pub mod record;

pub use backend::{AudioHost, AudioStream, CpalBackend, DeviceInfo};
pub use graph::{AudioNode, Connection, Graph, GraphProcessor, NodeId};
pub use record::RecordManager;
