//! Real-time audio engine with lock-free graph processing.

pub mod backend;
pub mod graph;
pub mod midi_io;
pub mod record;

pub use backend::{AudioHost, AudioStream, CpalBackend, DeviceInfo};
pub use graph::{AudioNode, Connection, Graph, GraphProcessor, NodeId};
pub use midi_io::{MidiPortInfo, enumerate_midi_ports};
pub use record::RecordManager;
