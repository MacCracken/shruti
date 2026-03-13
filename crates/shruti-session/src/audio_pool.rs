use std::collections::HashMap;
use std::path::Path;

use shruti_dsp::AudioBuffer;
use shruti_dsp::io::read_audio_file;

/// In-memory pool of audio files loaded into the session.
///
/// Audio files are loaded once and referenced by ID from regions.
/// The pool owns the decoded audio data.
///
/// When a `max_entries` limit is set, inserting beyond the limit evicts
/// the least-recently-used entry (based on an internal access counter).
pub struct AudioPool {
    buffers: HashMap<String, PoolEntry>,
    /// Monotonically increasing counter for LRU tracking.
    access_counter: u64,
    /// Maximum number of entries (0 = unlimited).
    max_entries: usize,
}

struct PoolEntry {
    buffer: AudioBuffer,
    last_access: u64,
}

impl AudioPool {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            access_counter: 0,
            max_entries: 0,
        }
    }

    /// Create an audio pool with a maximum entry limit.
    /// When inserting beyond this limit, the least-recently-used entry is evicted.
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            buffers: HashMap::new(),
            access_counter: 0,
            max_entries,
        }
    }

    /// Insert a pre-decoded buffer into the pool.
    pub fn insert(&mut self, id: String, buffer: AudioBuffer) {
        self.access_counter += 1;
        let entry = PoolEntry {
            buffer,
            last_access: self.access_counter,
        };
        self.buffers.insert(id, entry);
        self.evict_if_needed();
    }

    /// Load an audio file from disk and add it to the pool.
    /// Returns the assigned file ID.
    pub fn load(&mut self, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let id = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let (buffer, _format) = read_audio_file(path)?;
        self.insert(id.clone(), buffer);
        Ok(id)
    }

    /// Get a buffer by ID.
    pub fn get(&self, id: &str) -> Option<&AudioBuffer> {
        self.buffers.get(id).map(|e| &e.buffer)
    }

    /// Mark an entry as recently used (updates its LRU timestamp).
    pub fn touch(&mut self, id: &str) {
        if let Some(entry) = self.buffers.get_mut(id) {
            self.access_counter += 1;
            entry.last_access = self.access_counter;
        }
    }

    /// Remove a buffer by ID.
    pub fn remove(&mut self, id: &str) -> Option<AudioBuffer> {
        self.buffers.remove(id).map(|e| e.buffer)
    }

    /// List all file IDs in the pool.
    pub fn ids(&self) -> Vec<&str> {
        self.buffers.keys().map(|s| s.as_str()).collect()
    }

    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Evict the least-recently-used entry if we exceed `max_entries`.
    fn evict_if_needed(&mut self) {
        if self.max_entries == 0 || self.buffers.len() <= self.max_entries {
            return;
        }
        // Find the entry with the smallest last_access
        let lru_key = self
            .buffers
            .iter()
            .min_by_key(|(_, e)| e.last_access)
            .map(|(k, _)| k.clone());
        if let Some(key) = lru_key {
            self.buffers.remove(&key);
        }
    }
}

impl Default for AudioPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_unknown_key_returns_none() {
        let pool = AudioPool::new();
        assert!(pool.get("nonexistent").is_none());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let mut pool = AudioPool::new();
        let result = pool.load(Path::new("/tmp/does_not_exist_shruti_test.wav"));
        assert!(result.is_err());
        assert!(pool.is_empty());
    }

    #[test]
    fn test_load_wav_file() {
        // Create a small WAV file using hound
        let dir = tempfile::tempdir().unwrap();
        let wav_path = dir.path().join("test.wav");

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&wav_path, spec).unwrap();
        // Write 100 frames of silence (2 channels)
        for _ in 0..200 {
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();

        let mut pool = AudioPool::new();
        let id = pool.load(&wav_path).unwrap();
        assert_eq!(id, "test.wav");
        assert!(!pool.is_empty());
        assert_eq!(pool.len(), 1);

        let buf = pool.get("test.wav");
        assert!(buf.is_some());
    }

    #[test]
    fn test_multiple_loads() {
        let dir = tempfile::tempdir().unwrap();

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        // Create two different WAV files
        for name in &["a.wav", "b.wav"] {
            let p = dir.path().join(name);
            let mut w = hound::WavWriter::create(&p, spec).unwrap();
            for _ in 0..100 {
                w.write_sample(0i16).unwrap();
            }
            w.finalize().unwrap();
        }

        let mut pool = AudioPool::new();
        let id_a = pool.load(&dir.path().join("a.wav")).unwrap();
        let id_b = pool.load(&dir.path().join("b.wav")).unwrap();

        assert_eq!(id_a, "a.wav");
        assert_eq!(id_b, "b.wav");
        assert_eq!(pool.len(), 2);

        let mut ids = pool.ids();
        ids.sort();
        assert_eq!(ids, vec!["a.wav", "b.wav"]);
    }

    #[test]
    fn test_insert_and_remove() {
        let mut pool = AudioPool::new();
        let buf = AudioBuffer::new(2, 100);
        pool.insert("manual".into(), buf);
        assert_eq!(pool.len(), 1);
        assert!(pool.get("manual").is_some());

        let removed = pool.remove("manual");
        assert!(removed.is_some());
        assert!(pool.is_empty());
    }

    #[test]
    fn test_lru_eviction_evicts_oldest() {
        let mut pool = AudioPool::with_max_entries(2);
        pool.insert("a".into(), AudioBuffer::new(1, 10));
        pool.insert("b".into(), AudioBuffer::new(1, 10));
        // Pool is at capacity (2). Inserting a third should evict "a" (oldest).
        pool.insert("c".into(), AudioBuffer::new(1, 10));

        assert_eq!(pool.len(), 2);
        assert!(pool.get("a").is_none(), "a should have been evicted");
        assert!(pool.get("b").is_some());
        assert!(pool.get("c").is_some());
    }

    #[test]
    fn test_lru_touch_prevents_eviction() {
        let mut pool = AudioPool::with_max_entries(2);
        pool.insert("a".into(), AudioBuffer::new(1, 10));
        pool.insert("b".into(), AudioBuffer::new(1, 10));
        // Touch "a" so it's more recently used than "b"
        pool.touch("a");
        // Inserting "c" should evict "b" (now the LRU)
        pool.insert("c".into(), AudioBuffer::new(1, 10));

        assert_eq!(pool.len(), 2);
        assert!(pool.get("a").is_some(), "a was touched, should survive");
        assert!(pool.get("b").is_none(), "b should have been evicted");
        assert!(pool.get("c").is_some());
    }

    #[test]
    fn test_unlimited_pool_no_eviction() {
        let mut pool = AudioPool::new(); // max_entries = 0 (unlimited)
        for i in 0..100 {
            pool.insert(format!("file_{i}"), AudioBuffer::new(1, 10));
        }
        assert_eq!(pool.len(), 100);
    }

    #[test]
    fn test_touch_nonexistent_is_noop() {
        let mut pool = AudioPool::with_max_entries(2);
        pool.insert("a".into(), AudioBuffer::new(1, 10));
        pool.touch("nonexistent"); // should not panic
        assert_eq!(pool.len(), 1);
    }
}
