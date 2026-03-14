//! Lock-free stereo peak meter using atomics.
//!
//! Uses `AtomicU32` to store `f32` bit patterns so both the audio callback
//! (writer) and the UI thread (reader) can operate without any mutex.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// A single stereo peak level stored as two `AtomicU32` values.
///
/// The audio thread writes peak values; the UI thread reads them.
/// No mutex is involved — just relaxed atomic loads/stores on `f32` bits.
pub struct AtomicStereoLevel {
    left: AtomicU32,
    right: AtomicU32,
}

impl AtomicStereoLevel {
    pub fn new() -> Self {
        Self {
            left: AtomicU32::new(0.0f32.to_bits()),
            right: AtomicU32::new(0.0f32.to_bits()),
        }
    }

    /// Store a stereo peak pair (called from the audio thread).
    #[inline]
    pub fn store(&self, left: f32, right: f32) {
        self.left.store(left.to_bits(), Ordering::Relaxed);
        self.right.store(right.to_bits(), Ordering::Relaxed);
    }

    /// Load the current stereo peak pair (called from the UI thread).
    #[inline]
    pub fn load(&self) -> [f32; 2] {
        let l = f32::from_bits(self.left.load(Ordering::Relaxed));
        let r = f32::from_bits(self.right.load(Ordering::Relaxed));
        [l, r]
    }
}

impl Default for AtomicStereoLevel {
    fn default() -> Self {
        Self::new()
    }
}

/// Lock-free meter bank for multiple channels/slots.
///
/// Pre-allocates a fixed number of slots. The slot count can be grown
/// (but never shrunk) via `ensure_slots()`. The audio thread writes
/// into slots by index; the UI thread reads all slots.
pub struct MeterLevels {
    /// Fixed-capacity array of atomic stereo levels.
    slots: Vec<AtomicStereoLevel>,
    /// Number of active slots (can grow up to `slots.len()`).
    active: AtomicUsize,
}

impl MeterLevels {
    /// Create a meter bank with the given number of slots.
    pub fn new(count: usize) -> Self {
        let mut slots = Vec::with_capacity(count);
        for _ in 0..count {
            slots.push(AtomicStereoLevel::new());
        }
        Self {
            slots,
            active: AtomicUsize::new(count),
        }
    }

    /// Number of active meter slots.
    pub fn len(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }

    /// Whether there are no active slots.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Store a stereo peak for the given slot index.
    /// No-op if the index is out of bounds.
    #[inline]
    pub fn store(&self, index: usize, left: f32, right: f32) {
        if index < self.slots.len() {
            self.slots[index].store(left, right);
        }
    }

    /// Load the stereo peak for the given slot index.
    /// Returns `[0.0, 0.0]` if the index is out of bounds.
    #[inline]
    pub fn load(&self, index: usize) -> [f32; 2] {
        if index < self.slots.len() {
            self.slots[index].load()
        } else {
            [0.0, 0.0]
        }
    }

    /// Read all active slots into a Vec.
    pub fn read_all(&self) -> Vec<[f32; 2]> {
        let count = self.len();
        (0..count).map(|i| self.load(i)).collect()
    }

    /// Set the number of active slots. Cannot exceed the allocated capacity.
    pub fn set_active(&self, count: usize) {
        let clamped = count.min(self.slots.len());
        self.active.store(clamped, Ordering::Relaxed);
    }

    /// The total allocated capacity.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
}

/// Shared, reference-counted meter bank.
pub type SharedMeterLevels = Arc<MeterLevels>;

/// Create a new shared meter bank with the given slot count.
pub fn shared_meter_levels(count: usize) -> SharedMeterLevels {
    Arc::new(MeterLevels::new(count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_stereo_level_default() {
        let level = AtomicStereoLevel::new();
        assert_eq!(level.load(), [0.0, 0.0]);
    }

    #[test]
    fn test_atomic_stereo_level_store_load() {
        let level = AtomicStereoLevel::new();
        level.store(0.75, 0.5);
        let [l, r] = level.load();
        assert!((l - 0.75).abs() < 1e-7);
        assert!((r - 0.5).abs() < 1e-7);
    }

    #[test]
    fn test_atomic_stereo_level_negative_values() {
        let level = AtomicStereoLevel::new();
        level.store(-1.0, -0.5);
        let [l, r] = level.load();
        assert!((l - (-1.0)).abs() < 1e-7);
        assert!((r - (-0.5)).abs() < 1e-7);
    }

    #[test]
    fn test_atomic_stereo_level_overwrite() {
        let level = AtomicStereoLevel::new();
        level.store(1.0, 1.0);
        level.store(0.25, 0.1);
        let [l, r] = level.load();
        assert!((l - 0.25).abs() < 1e-7);
        assert!((r - 0.1).abs() < 1e-7);
    }

    #[test]
    fn test_meter_levels_new() {
        let meters = MeterLevels::new(4);
        assert_eq!(meters.len(), 4);
        assert_eq!(meters.capacity(), 4);
        assert!(!meters.is_empty());
    }

    #[test]
    fn test_meter_levels_empty() {
        let meters = MeterLevels::new(0);
        assert_eq!(meters.len(), 0);
        assert!(meters.is_empty());
    }

    #[test]
    fn test_meter_levels_store_and_load() {
        let meters = MeterLevels::new(3);
        meters.store(0, 0.5, 0.6);
        meters.store(1, 0.7, 0.8);
        meters.store(2, 0.9, 1.0);

        assert_eq!(meters.load(0), [0.5, 0.6]);
        assert_eq!(meters.load(1), [0.7, 0.8]);
        assert_eq!(meters.load(2), [0.9, 1.0]);
    }

    #[test]
    fn test_meter_levels_out_of_bounds_store() {
        let meters = MeterLevels::new(2);
        // Should not panic — just a no-op
        meters.store(5, 1.0, 1.0);
        assert_eq!(meters.load(5), [0.0, 0.0]);
    }

    #[test]
    fn test_meter_levels_out_of_bounds_load() {
        let meters = MeterLevels::new(1);
        assert_eq!(meters.load(99), [0.0, 0.0]);
    }

    #[test]
    fn test_meter_levels_read_all() {
        let meters = MeterLevels::new(3);
        meters.store(0, 0.1, 0.2);
        meters.store(1, 0.3, 0.4);
        meters.store(2, 0.5, 0.6);

        let all = meters.read_all();
        assert_eq!(all.len(), 3);
        assert!((all[0][0] - 0.1).abs() < 1e-7);
        assert!((all[2][1] - 0.6).abs() < 1e-7);
    }

    #[test]
    fn test_meter_levels_set_active() {
        let meters = MeterLevels::new(5);
        assert_eq!(meters.len(), 5);

        meters.set_active(3);
        assert_eq!(meters.len(), 3);

        // read_all should only return 3
        let all = meters.read_all();
        assert_eq!(all.len(), 3);

        // Cannot exceed capacity
        meters.set_active(100);
        assert_eq!(meters.len(), 5);
    }

    #[test]
    fn test_shared_meter_levels() {
        let shared = shared_meter_levels(2);
        shared.store(0, 0.5, 0.5);

        let clone = Arc::clone(&shared);
        assert_eq!(clone.load(0), [0.5, 0.5]);
    }

    #[test]
    fn test_meter_levels_cross_thread() {
        let meters = Arc::new(MeterLevels::new(2));
        let writer = Arc::clone(&meters);
        let reader = Arc::clone(&meters);

        let handle = std::thread::spawn(move || {
            for i in 0..1000 {
                let v = (i as f32) / 1000.0;
                writer.store(0, v, v);
            }
        });

        // Reader in main thread — should never panic or get garbage
        for _ in 0..1000 {
            let [l, r] = reader.load(0);
            assert!((0.0..=1.0).contains(&l));
            assert!((0.0..=1.0).contains(&r));
        }

        handle.join().unwrap();
    }

    #[test]
    fn test_atomic_stereo_level_default_trait() {
        let level = AtomicStereoLevel::default();
        assert_eq!(level.load(), [0.0, 0.0]);
    }

    #[test]
    fn test_meter_levels_set_active_to_zero() {
        let meters = MeterLevels::new(5);
        meters.store(0, 0.5, 0.5);
        meters.set_active(0);
        assert!(meters.is_empty());
        assert_eq!(meters.len(), 0);
        let all = meters.read_all();
        assert!(all.is_empty());
    }

    #[test]
    fn test_meter_levels_store_at_last_index() {
        let meters = MeterLevels::new(4);
        // Store at the boundary index (capacity - 1)
        meters.store(3, 0.9, 0.8);
        assert_eq!(meters.load(3), [0.9, 0.8]);
        // One past boundary is out of bounds
        meters.store(4, 1.0, 1.0);
        assert_eq!(meters.load(4), [0.0, 0.0]);
    }

    #[test]
    fn test_meter_levels_read_all_after_resize() {
        let meters = MeterLevels::new(4);
        meters.store(0, 0.1, 0.2);
        meters.store(1, 0.3, 0.4);
        meters.store(2, 0.5, 0.6);
        meters.store(3, 0.7, 0.8);

        meters.set_active(2);
        let all = meters.read_all();
        assert_eq!(all.len(), 2);
        assert!((all[0][0] - 0.1).abs() < 1e-7);
        assert!((all[1][0] - 0.3).abs() < 1e-7);
    }

    #[test]
    fn test_meter_levels_store_in_deactivated_range() {
        let meters = MeterLevels::new(4);
        meters.set_active(2);
        // Store in slot 3 which is deactivated but still within capacity
        meters.store(3, 0.9, 0.9);
        // The value is stored (capacity allows it) but read_all won't include it
        assert_eq!(meters.load(3), [0.9, 0.9]);
        let all = meters.read_all();
        assert_eq!(all.len(), 2); // only active slots
    }

    #[test]
    fn test_meter_levels_set_active_grow_back() {
        let meters = MeterLevels::new(4);
        meters.set_active(1);
        assert_eq!(meters.len(), 1);
        meters.set_active(4);
        assert_eq!(meters.len(), 4);
    }

    #[test]
    fn test_shared_meter_levels_arc_count() {
        let shared = shared_meter_levels(3);
        assert_eq!(Arc::strong_count(&shared), 1);
        let clone = Arc::clone(&shared);
        assert_eq!(Arc::strong_count(&shared), 2);
        drop(clone);
        assert_eq!(Arc::strong_count(&shared), 1);
    }
}
