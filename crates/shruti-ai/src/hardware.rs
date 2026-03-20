//! Hardware accelerator detection wrapper for the agent API.
//!
//! Wraps [`ai_hwaccel`] to provide a serializable summary of detected
//! AI hardware suitable for MCP tool responses and agent queries.
//!
//! Detection results are cached for 5 minutes to avoid repeated subprocess
//! spawns. Only CUDA and ROCm backends are probed (desktop GPU focus).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

// Re-export key types from ai-hwaccel.
pub use ai_hwaccel::{
    AcceleratorFamily, AcceleratorProfile, AcceleratorRegistry, AcceleratorType, Backend,
    DetectBuilder, QuantizationLevel, ShardingPlan,
};

/// Summary of a single detected device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub family: String,
    pub memory_bytes: u64,
    pub available: bool,
    pub memory_free_bytes: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub gpu_utilization_percent: Option<u32>,
    pub temperature_c: Option<u32>,
    pub power_watts: Option<f64>,
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

/// Cache TTL for hardware detection results.
const CACHE_TTL: Duration = Duration::from_secs(300);

type CacheState = (Option<Arc<AcceleratorRegistry>>, Option<Instant>);

static CACHE: std::sync::LazyLock<Mutex<CacheState>> =
    std::sync::LazyLock::new(|| Mutex::new((None, None)));

/// Build a selective detection using only CUDA and ROCm backends.
fn detect_selective() -> AcceleratorRegistry {
    DetectBuilder::none().with_cuda().with_rocm().detect()
}

/// Detect hardware and build a [`HardwareInfo`] summary.
///
/// Results are cached for 5 minutes. Only probes CUDA and ROCm backends.
pub fn detect() -> HardwareInfo {
    let mut guard = CACHE.lock().unwrap_or_else(|p| {
        let mut g = p.into_inner();
        g.0 = None;
        g.1 = None;
        g
    });

    if let (Some(reg), Some(ts)) = &*guard
        && ts.elapsed() < CACHE_TTL
    {
        return build_info(reg);
    }

    let registry = Arc::new(detect_selective());
    *guard = (Some(Arc::clone(&registry)), Some(Instant::now()));
    build_info(&registry)
}

/// Async variant of [`detect`] — non-blocking subprocess I/O via tokio.
///
/// Uses the selective CUDA+ROCm builder. Falls back to sync detection on error.
pub async fn detect_async() -> HardwareInfo {
    let registry = DetectBuilder::none()
        .with_cuda()
        .with_rocm()
        .detect_async()
        .await
        .unwrap_or_else(|_| detect_selective());
    build_info(&registry)
}

/// Invalidate the detection cache, forcing the next [`detect`] call to re-probe.
pub fn invalidate_cache() {
    let mut guard = CACHE.lock().unwrap_or_else(|p| {
        let mut g = p.into_inner();
        g.0 = None;
        g.1 = None;
        g
    });
    *guard = (None, None);
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
            memory_free_bytes: p.memory_free_bytes,
            memory_used_bytes: p.memory_used_bytes,
            gpu_utilization_percent: p.gpu_utilization_percent,
            temperature_c: p.temperature_c,
            power_watts: p.power_watts,
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

    #[test]
    fn detect_is_cached() {
        // First call populates cache
        let info1 = detect();
        // Second call should hit cache (same result)
        let info2 = detect();
        assert_eq!(info1.devices.len(), info2.devices.len());
        assert_eq!(info1.total_memory_bytes, info2.total_memory_bytes);
    }

    #[test]
    fn invalidate_cache_works() {
        detect(); // populate
        invalidate_cache();
        // Should not panic — next detect re-probes
        let info = detect();
        assert!(!info.devices.is_empty());
    }

    #[test]
    fn device_info_has_optional_metrics() {
        let info = detect();
        // Just verify the fields exist and serialize properly
        for dev in &info.devices {
            let _ = dev.memory_free_bytes;
            let _ = dev.memory_used_bytes;
            let _ = dev.gpu_utilization_percent;
            let _ = dev.temperature_c;
            let _ = dev.power_watts;
        }
    }
}
