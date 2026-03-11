use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::instance::ParamId;

/// Serializable plugin state for save/restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginState {
    /// Plugin identifier (format-specific unique ID).
    pub plugin_id: String,
    /// Parameter values keyed by parameter ID.
    pub params: HashMap<ParamId, f64>,
    /// Opaque binary state from the plugin (format-specific chunk data).
    #[serde(with = "base64_bytes")]
    pub chunk: Vec<u8>,
}

impl PluginState {
    pub fn new(plugin_id: String) -> Self {
        Self {
            plugin_id,
            params: HashMap::new(),
            chunk: Vec::new(),
        }
    }
}

/// Serde helper for base64-encoding binary data in JSON.
mod base64_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        // Simple hex encoding (no external base64 crate needed)
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        hex.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let hex = String::deserialize(d)?;
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(serde::de::Error::custom))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_serialization() {
        let mut state = PluginState::new("com.example.reverb".into());
        state.params.insert(0, 0.5);
        state.params.insert(1, 0.8);
        state.chunk = vec![0xDE, 0xAD, 0xBE, 0xEF];

        let json = serde_json::to_string(&state).unwrap();
        let restored: PluginState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.plugin_id, "com.example.reverb");
        assert_eq!(restored.params[&0], 0.5);
        assert_eq!(restored.params[&1], 0.8);
        assert_eq!(restored.chunk, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }
}
