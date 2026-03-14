//! Real-time audio engine with lock-free graph processing.

pub mod backend;
pub mod error;
pub mod graph;
pub mod meter;
pub mod midi_io;
pub mod record;

pub use backend::{AudioHost, AudioStream, CpalBackend, DeviceInfo};
pub use error::EngineError;
pub use graph::{AudioNode, Connection, Graph, GraphProcessor, NodeId};
pub use meter::{MeterLevels, SharedMeterLevels, shared_meter_levels};
pub use midi_io::{MidiPortInfo, enumerate_midi_ports};
pub use record::RecordManager;
