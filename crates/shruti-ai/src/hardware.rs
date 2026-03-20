//! Hardware accelerator detection wrapper for the agent API.
//!
//! Wraps [`ai_hwaccel`] to provide a serializable summary of detected
//! AI hardware suitable for MCP tool responses and agent queries.

use serde::{Deserialize, Serialize};

// Re-export key types from ai-hwaccel.
pub use ai_hwaccel::{
    AcceleratorFamily, AcceleratorProfile, AcceleratorRegistry, AcceleratorType, QuantizationLevel,
    ShardingPlan,
};

/// Summary of a single detected device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub family: String,
    pub memory_bytes: u64,
    pub available: bool,
}

/// Serializable summary of detected AI hardware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub has_accelerator: bool,
    pub total_memory_bytes: u64,
    pub accelerator_memory_bytes: u64,
    pub best_device: Option<String>,
    pub suggested_quantization: String,
    pub devices: Vec<DeviceInfo>,
}

/// 7B parameters — the Phase 9 music LLM target.
const MODEL_PARAMS_7B: u64 = 7_000_000_000;

/// Detect hardware and build a [`HardwareInfo`] summary.
pub fn detect() -> HardwareInfo {
    let registry = AcceleratorRegistry::detect();
    build_info(&registry)
}

/// Build a [`HardwareInfo`] from an existing registry (useful for testing).
fn build_info(registry: &AcceleratorRegistry) -> HardwareInfo {
    let best = registry.best_available();
    let quant = registry.suggest_quantization(MODEL_PARAMS_7B);

    let devices: Vec<DeviceInfo> = registry
        .all_profiles()
        .iter()
        .map(|p| DeviceInfo {
            name: p.accelerator.to_string(),
            family: p.accelerator.family().to_string(),
            memory_bytes: p.memory_bytes,
            available: p.available,
        })
        .collect();

    HardwareInfo {
        has_accelerator: registry.has_accelerator(),
        total_memory_bytes: registry.total_memory(),
        accelerator_memory_bytes: registry.total_accelerator_memory(),
        best_device: best.map(|p| p.to_string()),
        suggested_quantization: quant.to_string(),
        devices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_valid_info() {
        let info = detect();
        // At minimum, CPU is always present.
        assert!(!info.devices.is_empty());
        assert!(info.total_memory_bytes > 0);
        assert!(!info.suggested_quantization.is_empty());
        // CPU device should exist.
        assert!(info.devices.iter().any(|d| d.family == "CPU"));
    }

    #[test]
    fn hardware_info_serializes() {
        let info = detect();
        let json = serde_json::to_string(&info).expect("serialize");
        let roundtrip: HardwareInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtrip.has_accelerator, info.has_accelerator);
        assert_eq!(roundtrip.total_memory_bytes, info.total_memory_bytes);
        assert_eq!(roundtrip.devices.len(), info.devices.len());
    }
}
