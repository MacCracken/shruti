use std::collections::HashMap;
use std::path::Path;

use crate::format::PluginFormat;
use crate::instance::{PluginInfo, PluginInstance};
use crate::scanner::{PluginScanner, ScannedPlugin};
use crate::state::PluginState;

/// The plugin host manages loading, activating, and unloading plugin instances.
pub struct PluginHost {
    scanner: PluginScanner,
    /// Registry of scanned plugins (populated by scan()).
    registry: Vec<ScannedPlugin>,
    /// Active plugin instances keyed by a user-assigned slot ID.
    instances: HashMap<String, Box<dyn PluginInstance>>,
}

impl PluginHost {
    pub fn new() -> Self {
        Self {
            scanner: PluginScanner::new(),
            registry: Vec::new(),
            instances: HashMap::new(),
        }
    }

    /// Add an additional search path.
    pub fn add_search_path(&mut self, path: impl Into<std::path::PathBuf>) {
        self.scanner.add_path(path);
    }

    /// Scan all search paths for plugins.
    pub fn scan(&mut self) -> &[ScannedPlugin] {
        self.registry = self.scanner.scan_all();
        &self.registry
    }

    /// Get the current plugin registry (from last scan).
    pub fn registry(&self) -> &[ScannedPlugin] {
        &self.registry
    }

    /// Find a scanned plugin by name.
    pub fn find_plugin(&self, name: &str) -> Option<&ScannedPlugin> {
        self.registry.iter().find(|p| p.name == name)
    }

    /// Load a plugin from a scanned entry into a named slot.
    ///
    /// The actual loading depends on the plugin format:
    /// - CLAP: Loads the shared library and calls clap_entry
    /// - VST3: Loads the VST3 bundle
    /// - Native: Loads the shared library and calls the Shruti plugin entry point
    pub fn load(
        &mut self,
        slot: &str,
        plugin: &ScannedPlugin,
        sample_rate: f64,
        max_buffer_size: u32,
    ) -> Result<&dyn PluginInstance, String> {
        let instance = load_plugin(plugin, sample_rate, max_buffer_size)?;
        self.instances.insert(slot.to_string(), instance);
        Ok(self.instances.get(slot).unwrap().as_ref())
    }

    /// Unload a plugin from a named slot.
    pub fn unload(&mut self, slot: &str) -> Option<Box<dyn PluginInstance>> {
        if let Some(mut inst) = self.instances.remove(slot) {
            inst.deactivate();
            Some(inst)
        } else {
            None
        }
    }

    /// Get a reference to an active plugin instance.
    pub fn instance(&self, slot: &str) -> Option<&dyn PluginInstance> {
        self.instances.get(slot).map(|i| i.as_ref())
    }

    /// Get a mutable reference to an active plugin instance.
    pub fn instance_mut(&mut self, slot: &str) -> Option<&mut Box<dyn PluginInstance>> {
        self.instances.get_mut(slot)
    }

    /// Save state of all active plugins.
    pub fn save_all_states(&self) -> HashMap<String, PluginState> {
        self.instances
            .iter()
            .map(|(slot, inst)| (slot.clone(), inst.save_state()))
            .collect()
    }

    /// Restore states for active plugins.
    pub fn load_all_states(&mut self, states: &HashMap<String, PluginState>) {
        for (slot, state) in states {
            if let Some(inst) = self.instances.get_mut(slot) {
                inst.load_state(state);
            }
        }
    }

    /// List all active plugin slots.
    pub fn active_slots(&self) -> Vec<&str> {
        self.instances.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}

/// Load a plugin instance based on its format.
fn load_plugin(
    plugin: &ScannedPlugin,
    sample_rate: f64,
    max_buffer_size: u32,
) -> Result<Box<dyn PluginInstance>, String> {
    match plugin.format {
        PluginFormat::Clap => load_clap_plugin(&plugin.path, sample_rate, max_buffer_size),
        PluginFormat::Vst3 => load_vst3_plugin(&plugin.path, sample_rate, max_buffer_size),
        PluginFormat::Native => load_native_plugin(&plugin.path, sample_rate, max_buffer_size),
    }
}

fn load_clap_plugin(
    path: &Path,
    sample_rate: f64,
    max_buffer_size: u32,
) -> Result<Box<dyn PluginInstance>, String> {
    // Load the shared library
    let lib = unsafe { libloading::Library::new(path) }
        .map_err(|e| format!("failed to load CLAP plugin {}: {e}", path.display()))?;

    // CLAP entry point: clap_entry
    let _entry: libloading::Symbol<*const ()> = unsafe { lib.get(b"clap_entry\0") }
        .map_err(|e| format!("CLAP entry point not found in {}: {e}", path.display()))?;

    // Create a stub instance that holds the library handle
    // Full CLAP host protocol implementation would go here
    let info = PluginInfo {
        id: format!("clap:{}", path.display()),
        name: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string(),
        vendor: String::new(),
        version: String::new(),
        format: PluginFormat::Clap,
        path: path.to_string_lossy().into(),
        num_audio_inputs: 2,
        num_audio_outputs: 2,
        has_gui: false,
    };

    Ok(Box::new(LoadedPlugin::new(
        info,
        lib,
        sample_rate,
        max_buffer_size,
    )))
}

fn load_vst3_plugin(
    path: &Path,
    sample_rate: f64,
    max_buffer_size: u32,
) -> Result<Box<dyn PluginInstance>, String> {
    // VST3 bundles: find the actual shared library inside the bundle
    let lib_path = find_vst3_binary(path)?;

    let lib = unsafe { libloading::Library::new(&lib_path) }
        .map_err(|e| format!("failed to load VST3 plugin {}: {e}", lib_path.display()))?;

    // VST3 entry point: GetPluginFactory
    let _factory: libloading::Symbol<*const ()> = unsafe { lib.get(b"GetPluginFactory\0") }
        .map_err(|e| format!("VST3 entry point not found in {}: {e}", lib_path.display()))?;

    let info = PluginInfo {
        id: format!("vst3:{}", path.display()),
        name: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string(),
        vendor: String::new(),
        version: String::new(),
        format: PluginFormat::Vst3,
        path: path.to_string_lossy().into(),
        num_audio_inputs: 2,
        num_audio_outputs: 2,
        has_gui: false,
    };

    Ok(Box::new(LoadedPlugin::new(
        info,
        lib,
        sample_rate,
        max_buffer_size,
    )))
}

fn load_native_plugin(
    path: &Path,
    sample_rate: f64,
    max_buffer_size: u32,
) -> Result<Box<dyn PluginInstance>, String> {
    let lib = unsafe { libloading::Library::new(path) }
        .map_err(|e| format!("failed to load native plugin {}: {e}", path.display()))?;

    // Native entry point: shruti_plugin_create
    let _create: libloading::Symbol<*const ()> = unsafe { lib.get(b"shruti_plugin_create\0") }
        .map_err(|e| {
            format!(
                "native plugin entry point not found in {}: {e}",
                path.display()
            )
        })?;

    let info = PluginInfo {
        id: format!("native:{}", path.display()),
        name: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string(),
        vendor: String::new(),
        version: String::new(),
        format: PluginFormat::Native,
        path: path.to_string_lossy().into(),
        num_audio_inputs: 2,
        num_audio_outputs: 2,
        has_gui: false,
    };

    Ok(Box::new(LoadedPlugin::new(
        info,
        lib,
        sample_rate,
        max_buffer_size,
    )))
}

/// Find the platform-specific binary inside a VST3 bundle directory.
fn find_vst3_binary(bundle: &Path) -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "linux")]
    let arch_dir = if cfg!(target_arch = "x86_64") {
        "x86_64-linux"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64-linux"
    } else {
        "x86_64-linux"
    };

    #[cfg(target_os = "macos")]
    let arch_dir = "MacOS";

    #[cfg(target_os = "windows")]
    let arch_dir = if cfg!(target_arch = "x86_64") {
        "x86_64-win"
    } else {
        "x86_64-win"
    };

    let name = bundle
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("plugin");

    #[cfg(target_os = "linux")]
    let binary = bundle
        .join("Contents")
        .join(arch_dir)
        .join(format!("{name}.so"));

    #[cfg(target_os = "macos")]
    let binary = bundle.join("Contents").join(arch_dir).join(name);

    #[cfg(target_os = "windows")]
    let binary = bundle
        .join("Contents")
        .join(arch_dir)
        .join(format!("{name}.vst3"));

    if binary.exists() {
        Ok(binary)
    } else {
        Err(format!("VST3 binary not found at {}", binary.display()))
    }
}

/// A loaded plugin instance backed by a shared library.
///
/// This is the unified wrapper around CLAP/VST3/Native plugins.
/// Full format-specific protocol handling (CLAP host callbacks, VST3 COM interfaces)
/// would extend this with per-format dispatch.
struct LoadedPlugin {
    info: PluginInfo,
    _lib: libloading::Library,
    params: Vec<crate::instance::ParamInfo>,
    param_values: std::collections::HashMap<crate::instance::ParamId, f64>,
    active: bool,
    _sample_rate: f64,
    _max_buffer_size: u32,
}

impl LoadedPlugin {
    fn new(
        info: PluginInfo,
        lib: libloading::Library,
        sample_rate: f64,
        max_buffer_size: u32,
    ) -> Self {
        Self {
            info,
            _lib: lib,
            params: Vec::new(),
            param_values: std::collections::HashMap::new(),
            active: false,
            _sample_rate: sample_rate,
            _max_buffer_size: max_buffer_size,
        }
    }
}

impl PluginInstance for LoadedPlugin {
    fn info(&self) -> &PluginInfo {
        &self.info
    }

    fn activate(&mut self, _sample_rate: f64, _max_buffer_size: u32) -> Result<(), String> {
        self.active = true;
        Ok(())
    }

    fn deactivate(&mut self) {
        self.active = false;
    }

    fn process(&mut self, input: &shruti_dsp::AudioBuffer, output: &mut shruti_dsp::AudioBuffer) {
        // Pass-through until format-specific processing is wired
        let src = input.as_interleaved();
        let dst = output.as_interleaved_mut();
        let len = src.len().min(dst.len());
        dst[..len].copy_from_slice(&src[..len]);
    }

    fn params(&self) -> Vec<crate::instance::ParamInfo> {
        self.params.clone()
    }

    fn get_param(&self, id: crate::instance::ParamId) -> f64 {
        self.param_values.get(&id).copied().unwrap_or(0.0)
    }

    fn set_param(&mut self, id: crate::instance::ParamId, value: f64) {
        self.param_values.insert(id, value);
    }

    fn save_state(&self) -> crate::state::PluginState {
        crate::state::PluginState {
            plugin_id: self.info.id.clone(),
            params: self.param_values.clone(),
            chunk: Vec::new(),
        }
    }

    fn load_state(&mut self, state: &crate::state::PluginState) {
        self.param_values = state.params.clone();
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_host_empty() {
        let host = PluginHost::new();
        assert!(host.registry().is_empty());
        assert!(host.active_slots().is_empty());
    }

    #[test]
    fn default_is_new() {
        let host = PluginHost::default();
        assert!(host.registry().is_empty());
    }

    #[test]
    fn find_plugin_empty_registry() {
        let host = PluginHost::new();
        assert!(host.find_plugin("nonexistent").is_none());
    }

    #[test]
    fn instance_empty() {
        let host = PluginHost::new();
        assert!(host.instance("slot1").is_none());
    }

    #[test]
    fn instance_mut_empty() {
        let mut host = PluginHost::new();
        assert!(host.instance_mut("slot1").is_none());
    }

    #[test]
    fn unload_nonexistent_slot() {
        let mut host = PluginHost::new();
        assert!(host.unload("nothing").is_none());
    }

    #[test]
    fn save_all_states_empty() {
        let host = PluginHost::new();
        let states = host.save_all_states();
        assert!(states.is_empty());
    }

    #[test]
    fn load_all_states_empty() {
        let mut host = PluginHost::new();
        let states = HashMap::new();
        host.load_all_states(&states); // should not panic
    }

    #[test]
    fn scan_empty_paths() {
        let mut host = PluginHost::new();
        let results = host.scan();
        // No real plugin directories — expect empty or whatever system finds
        // Just verify it doesn't panic
        let _ = results.len();
    }

    #[test]
    fn add_search_path() {
        let mut host = PluginHost::new();
        host.add_search_path("/tmp/fake-plugins");
        // Just verify it doesn't panic
    }

    #[test]
    fn find_vst3_binary_missing() {
        let result = find_vst3_binary(Path::new("/tmp/nonexistent.vst3"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("VST3 binary not found"));
    }

    #[test]
    fn loaded_plugin_lifecycle() {
        // We can't create a LoadedPlugin without a real Library handle,
        // but we can test the load function error paths
        let fake_plugin = ScannedPlugin {
            name: "FakePlugin".into(),
            path: "/tmp/nonexistent.clap".into(),
            format: PluginFormat::Clap,
        };
        let result = load_plugin(&fake_plugin, 48000.0, 256);
        assert!(result.is_err());
    }

    #[test]
    fn load_clap_nonexistent() {
        let result = load_clap_plugin(Path::new("/nonexistent.clap"), 48000.0, 256);
        match result {
            Err(msg) => assert!(msg.contains("failed to load CLAP"), "got: {msg}"),
            Ok(_) => panic!("should have failed"),
        }
    }

    #[test]
    fn load_vst3_nonexistent() {
        let result = load_vst3_plugin(Path::new("/nonexistent.vst3"), 48000.0, 256);
        assert!(result.is_err());
    }

    #[test]
    fn load_native_nonexistent() {
        let result = load_native_plugin(Path::new("/nonexistent.so"), 48000.0, 256);
        match result {
            Err(msg) => assert!(msg.contains("failed to load native"), "got: {msg}"),
            Ok(_) => panic!("should have failed"),
        }
    }

    #[test]
    fn host_load_nonexistent_plugin() {
        let mut host = PluginHost::new();
        let fake = ScannedPlugin {
            name: "Ghost".into(),
            path: "/dev/null/nope.clap".into(),
            format: PluginFormat::Clap,
        };
        let result = host.load("slot1", &fake, 48000.0, 256);
        assert!(result.is_err());
        assert!(host.active_slots().is_empty());
    }
}
