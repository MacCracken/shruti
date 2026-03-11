use shruti_dsp::AudioBuffer;
use shruti_engine::graph::AudioNode;

use crate::instance::PluginInstance;

/// Audio graph node that wraps a plugin instance.
///
/// Integrates any PluginInstance into the shruti-engine audio graph,
/// allowing plugins to be used as processing nodes.
pub struct PluginNode {
    plugin: Box<dyn PluginInstance>,
    name: String,
}

impl PluginNode {
    pub fn new(plugin: Box<dyn PluginInstance>) -> Self {
        let name = format!("plugin:{}", plugin.info().name);
        Self { plugin, name }
    }

    /// Get a reference to the underlying plugin instance.
    pub fn plugin(&self) -> &dyn PluginInstance {
        self.plugin.as_ref()
    }

    /// Get a mutable reference to the underlying plugin instance.
    pub fn plugin_mut(&mut self) -> &mut dyn PluginInstance {
        self.plugin.as_mut()
    }
}

impl AudioNode for PluginNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn num_inputs(&self) -> usize {
        self.plugin.info().num_audio_inputs as usize
    }

    fn num_outputs(&self) -> usize {
        self.plugin.info().num_audio_outputs as usize
    }

    fn process(&mut self, inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
        if let Some(input) = inputs.first() {
            self.plugin.process(input, output);
        } else {
            // No input — provide silence
            let silent = AudioBuffer::new(output.channels(), output.frames());
            self.plugin.process(&silent, output);
        }
    }
}
