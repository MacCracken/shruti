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
