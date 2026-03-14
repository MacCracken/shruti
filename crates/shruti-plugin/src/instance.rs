use serde::{Deserialize, Serialize};
use shruti_dsp::AudioBuffer;

use crate::error::PluginError;
use crate::format::PluginFormat;
use crate::state::PluginState;

/// Unique parameter identifier within a plugin.
pub type ParamId = u32;

/// Metadata about a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub format: PluginFormat,
    pub path: String,
    pub num_audio_inputs: u32,
    pub num_audio_outputs: u32,
    pub has_gui: bool,
}

/// Metadata about a plugin parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamInfo {
    pub id: ParamId,
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub step: Option<f64>,
}

/// Trait for a loaded, active plugin instance.
///
/// Implementations wrap the format-specific plugin handle (CLAP, VST3, or native)
/// behind a unified interface.
pub trait PluginInstance: Send {
    /// Get plugin metadata.
    fn info(&self) -> &PluginInfo;

    /// Activate the plugin for processing at the given sample rate and buffer size.
    fn activate(&mut self, sample_rate: f64, max_buffer_size: u32) -> Result<(), PluginError>;

    /// Deactivate the plugin (stop processing).
    fn deactivate(&mut self);

    /// Process audio through the plugin.
    /// Reads from `input`, writes to `output`.
    fn process(&mut self, input: &AudioBuffer, output: &mut AudioBuffer);

    /// Get all parameter descriptors.
    fn params(&self) -> Vec<ParamInfo>;

    /// Get the current value of a parameter.
    fn get_param(&self, id: ParamId) -> f64;

    /// Set the value of a parameter.
    fn set_param(&mut self, id: ParamId, value: f64);

    /// Save the plugin's full state (parameters + internal state).
    fn save_state(&self) -> PluginState;

    /// Restore the plugin's state.
    fn load_state(&mut self, state: &PluginState);

    /// Whether the plugin is currently activated for processing.
    fn is_active(&self) -> bool;
}
