//! Plugin hosting for VST3, CLAP, and native Rust plugins.

pub mod format;
pub mod host;
pub mod instance;
pub mod node;
pub mod scanner;
pub mod state;

pub use format::PluginFormat;
pub use host::PluginHost;
pub use instance::{ParamId, ParamInfo, PluginInfo, PluginInstance};
pub use node::PluginNode;
pub use scanner::{PluginScanner, ScannedPlugin};
pub use state::PluginState;
