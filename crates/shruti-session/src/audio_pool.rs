use std::collections::HashMap;
use std::path::Path;

use shruti_dsp::AudioBuffer;
use shruti_dsp::io::read_audio_file;

/// In-memory pool of audio files loaded into the session.
///
/// Audio files are loaded once and referenced by ID from regions.
/// The pool owns the decoded audio data.
pub struct AudioPool {
    buffers: HashMap<String, AudioBuffer>,
}

impl AudioPool {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// Insert a pre-decoded buffer into the pool.
    pub fn insert(&mut self, id: String, buffer: AudioBuffer) {
        self.buffers.insert(id, buffer);
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
        self.buffers.insert(id.clone(), buffer);
        Ok(id)
    }

    /// Get a buffer by ID.
    pub fn get(&self, id: &str) -> Option<&AudioBuffer> {
        self.buffers.get(id)
    }

    /// Remove a buffer by ID.
    pub fn remove(&mut self, id: &str) -> Option<AudioBuffer> {
        self.buffers.remove(id)
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
}
