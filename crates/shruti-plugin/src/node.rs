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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PluginError;
    use crate::format::PluginFormat;
    use crate::instance::{ParamInfo, PluginInfo};
    use crate::state::PluginState;

    /// A test plugin that passes audio through unchanged.
    struct TestPlugin {
        info: PluginInfo,
        active: bool,
    }

    impl TestPlugin {
        fn new() -> Self {
            Self {
                info: PluginInfo {
                    id: "test:passthrough".into(),
                    name: "Passthrough".into(),
                    vendor: "Test".into(),
                    version: "1.0".into(),
                    format: PluginFormat::Native,
                    path: "/test".into(),
                    num_audio_inputs: 2,
                    num_audio_outputs: 2,
                    has_gui: false,
                },
                active: false,
            }
        }
    }

    impl PluginInstance for TestPlugin {
        fn info(&self) -> &PluginInfo {
            &self.info
        }
        fn activate(&mut self, _sr: f64, _bs: u32) -> Result<(), PluginError> {
            self.active = true;
            Ok(())
        }
        fn deactivate(&mut self) {
            self.active = false;
        }
        fn process(&mut self, input: &AudioBuffer, output: &mut AudioBuffer) {
            let src = input.as_interleaved();
            let dst = output.as_interleaved_mut();
            let len = src.len().min(dst.len());
            dst[..len].copy_from_slice(&src[..len]);
        }
        fn params(&self) -> Vec<ParamInfo> {
            vec![]
        }
        fn get_param(&self, _id: u32) -> f64 {
            0.0
        }
        fn set_param(&mut self, _id: u32, _value: f64) {}
        fn save_state(&self) -> PluginState {
            PluginState::new(self.info.id.clone())
        }
        fn load_state(&mut self, _state: &PluginState) {}
        fn is_active(&self) -> bool {
            self.active
        }
    }

    #[test]
    fn plugin_node_name_contains_plugin_name() {
        let plugin = TestPlugin::new();
        let node = PluginNode::new(Box::new(plugin));
        assert!(node.name().contains("Passthrough"));
    }

    #[test]
    fn plugin_node_num_inputs_outputs() {
        let plugin = TestPlugin::new();
        let node = PluginNode::new(Box::new(plugin));
        assert_eq!(node.num_inputs(), 2);
        assert_eq!(node.num_outputs(), 2);
    }

    #[test]
    fn plugin_node_process_with_input() {
        let plugin = TestPlugin::new();
        let mut node = PluginNode::new(Box::new(plugin));
        let input = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3], 2);
        let mut output = AudioBuffer::new(2, 2);
        node.process(&[&input], &mut output);
        assert_eq!(output.get(0, 0), 0.5);
        assert_eq!(output.get(0, 1), -0.5);
    }

    #[test]
    fn plugin_node_process_without_input() {
        let plugin = TestPlugin::new();
        let mut node = PluginNode::new(Box::new(plugin));
        let mut output = AudioBuffer::new(2, 4);
        node.process(&[], &mut output);
        // With no input, silent buffer is passed — output should be zeros
        for &s in output.as_interleaved() {
            assert_eq!(s, 0.0);
        }
    }

    #[test]
    fn plugin_node_plugin_ref() {
        let plugin = TestPlugin::new();
        let node = PluginNode::new(Box::new(plugin));
        assert_eq!(node.plugin().info().name, "Passthrough");
    }

    #[test]
    fn plugin_node_plugin_mut_ref() {
        let plugin = TestPlugin::new();
        let mut node = PluginNode::new(Box::new(plugin));
        assert!(!node.plugin().is_active());
        node.plugin_mut().activate(48000.0, 256).unwrap();
        assert!(node.plugin().is_active());
    }
}
