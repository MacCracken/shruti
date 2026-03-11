use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Application preferences persisted between sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    /// Preferred audio device name (None = system default).
    pub audio_device: Option<String>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Buffer size in frames.
    pub buffer_size: u32,
    /// Default project directory.
    pub project_dir: Option<PathBuf>,
    /// Recently opened session paths (most recent first).
    pub recent_sessions: Vec<PathBuf>,
    /// Maximum number of recent sessions to remember.
    pub max_recent: usize,
    /// UI scale factor (1.0 = 100%).
    pub ui_scale: f32,
    /// Theme file path (None = default theme).
    pub theme_path: Option<PathBuf>,
    /// Auto-save interval in seconds (0 = disabled).
    pub auto_save_interval: u32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            audio_device: None,
            sample_rate: 48000,
            buffer_size: 256,
            project_dir: None,
            recent_sessions: Vec::new(),
            max_recent: 10,
            ui_scale: 1.0,
            theme_path: None,
            auto_save_interval: 60,
        }
    }
}

impl Preferences {
    /// Load preferences from a JSON file.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(path)?;
        let prefs = serde_json::from_str(&data)?;
        Ok(prefs)
    }

    /// Save preferences to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Get the default preferences file path.
    pub fn default_path() -> PathBuf {
        let config_dir = dirs_or_fallback();
        config_dir.join("shruti").join("preferences.json")
    }

    /// Load from default path, or return defaults if not found.
    pub fn load_or_default() -> Self {
        let path = Self::default_path();
        Self::load(&path).unwrap_or_default()
    }

    /// Add a path to recent sessions.
    pub fn add_recent(&mut self, path: PathBuf) {
        // Remove if already present
        self.recent_sessions.retain(|p| p != &path);
        // Add to front
        self.recent_sessions.insert(0, path);
        // Trim
        self.recent_sessions.truncate(self.max_recent);
    }
}

fn dirs_or_fallback() -> PathBuf {
    // Use XDG_CONFIG_HOME on Linux, fallback to ~/.config
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config");
    }
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preferences_default() {
        let prefs = Preferences::default();
        assert_eq!(prefs.sample_rate, 48000);
        assert_eq!(prefs.buffer_size, 256);
        assert_eq!(prefs.ui_scale, 1.0);
        assert!(prefs.recent_sessions.is_empty());
    }

    #[test]
    fn test_preferences_roundtrip() {
        let mut prefs = Preferences::default();
        prefs.sample_rate = 44100;
        prefs.audio_device = Some("My Interface".into());
        prefs.add_recent(PathBuf::from("/projects/song1.shruti"));
        prefs.add_recent(PathBuf::from("/projects/song2.shruti"));

        let tmp = std::env::temp_dir().join("shruti_test_prefs.json");
        prefs.save(&tmp).unwrap();

        let loaded = Preferences::load(&tmp).unwrap();
        assert_eq!(loaded.sample_rate, 44100);
        assert_eq!(loaded.audio_device, Some("My Interface".into()));
        assert_eq!(loaded.recent_sessions.len(), 2);
        assert_eq!(
            loaded.recent_sessions[0],
            PathBuf::from("/projects/song2.shruti")
        );

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_recent_sessions() {
        let mut prefs = Preferences::default();
        prefs.max_recent = 3;

        prefs.add_recent(PathBuf::from("/a.shruti"));
        prefs.add_recent(PathBuf::from("/b.shruti"));
        prefs.add_recent(PathBuf::from("/c.shruti"));
        prefs.add_recent(PathBuf::from("/d.shruti"));

        assert_eq!(prefs.recent_sessions.len(), 3);
        assert_eq!(prefs.recent_sessions[0], PathBuf::from("/d.shruti"));

        // Re-adding existing moves to front
        prefs.add_recent(PathBuf::from("/b.shruti"));
        assert_eq!(prefs.recent_sessions[0], PathBuf::from("/b.shruti"));
        assert_eq!(prefs.recent_sessions.len(), 3);
    }
}
