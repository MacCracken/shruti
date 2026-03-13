use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Recording configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    /// Recording sample rate in Hz (e.g. 44100, 48000, 88200, 96000, 176400, 192000).
    pub sample_rate: u32,
    /// Number of recording channels (1 = mono, 2 = stereo, up to 8).
    pub channels: u16,
    /// Maximum recording duration in seconds (0 = unlimited up to memory cap).
    pub max_duration_secs: u32,
    /// Recording buffer size in frames (for the input stream callback).
    pub buffer_size: u32,
    /// Preferred input device name (None = system default).
    pub input_device: Option<String>,
}

impl RecordingConfig {
    /// Standard recording rates supported.
    pub const SUPPORTED_RATES: &[u32] = &[44100, 48000, 88200, 96000, 176400, 192000];

    /// Maximum allowed channels.
    pub const MAX_CHANNELS: u16 = 8;

    /// Calculate the maximum number of samples this config allows in the buffer.
    /// Returns the cap in total interleaved samples (frames * channels).
    pub fn max_buffer_samples(&self) -> usize {
        let duration = if self.max_duration_secs == 0 {
            1800 // Default 30 min if unlimited
        } else {
            self.max_duration_secs as usize
        };
        self.sample_rate as usize * self.channels as usize * duration
    }

    /// Validate the config, clamping values to safe ranges.
    pub fn validated(mut self) -> Self {
        if !Self::SUPPORTED_RATES.contains(&self.sample_rate) {
            // Snap to nearest supported rate
            self.sample_rate = *Self::SUPPORTED_RATES
                .iter()
                .min_by_key(|&&r| (r as i64 - self.sample_rate as i64).unsigned_abs())
                .unwrap_or(&48000);
        }
        self.channels = self.channels.clamp(1, Self::MAX_CHANNELS);
        self.buffer_size = self.buffer_size.clamp(64, 4096);
        self
    }
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            max_duration_secs: 1800, // 30 minutes
            buffer_size: 256,
            input_device: None,
        }
    }
}

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
    /// Recording configuration.
    #[serde(default)]
    pub recording: RecordingConfig,
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
            recording: RecordingConfig::default(),
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
    ///
    /// On Unix, the file permissions are set to 0600 (owner read/write only)
    /// to protect sensitive settings like device names and paths.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, &data)?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

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
        let mut prefs = Preferences {
            sample_rate: 44100,
            audio_device: Some("My Interface".into()),
            ..Default::default()
        };
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
        let mut prefs = Preferences {
            max_recent: 3,
            ..Default::default()
        };

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

    #[test]
    fn test_default_path_returns_a_path() {
        let path = Preferences::default_path();
        // Should end with the expected filename
        assert!(path.ends_with("shruti/preferences.json"));
        // Should be an absolute path or at least have components
        assert!(path.components().count() > 1);
    }

    #[test]
    fn test_add_recent_deduplication() {
        let mut prefs = Preferences::default();
        prefs.add_recent(PathBuf::from("/x.shruti"));
        prefs.add_recent(PathBuf::from("/y.shruti"));
        prefs.add_recent(PathBuf::from("/x.shruti")); // duplicate

        // Should have only 2 entries, not 3
        assert_eq!(prefs.recent_sessions.len(), 2);
        // The duplicate should be moved to the front
        assert_eq!(prefs.recent_sessions[0], PathBuf::from("/x.shruti"));
        assert_eq!(prefs.recent_sessions[1], PathBuf::from("/y.shruti"));
    }

    #[test]
    fn test_load_bad_json() {
        let dir = tempfile::tempdir().unwrap();
        let bad_path = dir.path().join("bad.json");
        std::fs::write(&bad_path, "this is not valid json{{{").unwrap();

        let result = Preferences::load(&bad_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = Preferences::load(Path::new("/tmp/nonexistent_shruti_prefs_xyz.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_save_to_nested_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested_path = dir
            .path()
            .join("deeply")
            .join("nested")
            .join("dir")
            .join("prefs.json");

        let prefs = Preferences::default();
        // The parent directories don't exist yet; save should create them
        let result = prefs.save(&nested_path);
        assert!(result.is_ok());
        assert!(nested_path.exists());

        // Verify the file is valid JSON and roundtrips
        let loaded = Preferences::load(&nested_path).unwrap();
        assert_eq!(loaded.sample_rate, prefs.sample_rate);
        assert_eq!(loaded.buffer_size, prefs.buffer_size);
    }

    #[test]
    fn test_load_or_default_missing_file() {
        // load_or_default should return defaults if the file doesn't exist
        // We can't easily control default_path, but we know it falls back
        let prefs = Preferences::load_or_default();
        // Should have default values (if no prefs file exists at default path)
        // At minimum, this should not panic
        assert!(prefs.sample_rate > 0);
    }

    // -- RecordingConfig tests ------------------------------------------------

    #[test]
    fn recording_config_defaults() {
        let cfg = RecordingConfig::default();
        assert_eq!(cfg.sample_rate, 48000);
        assert_eq!(cfg.channels, 2);
        assert_eq!(cfg.max_duration_secs, 1800);
        assert_eq!(cfg.buffer_size, 256);
        assert!(cfg.input_device.is_none());
    }

    #[test]
    fn recording_config_max_buffer_samples() {
        let cfg = RecordingConfig::default();
        // 48000 * 2 * 1800 = 172_800_000
        assert_eq!(cfg.max_buffer_samples(), 48000 * 2 * 1800);
    }

    #[test]
    fn recording_config_max_buffer_unlimited_caps_at_30_min() {
        let cfg = RecordingConfig {
            max_duration_secs: 0,
            ..Default::default()
        };
        // unlimited -> defaults to 1800s internally
        assert_eq!(cfg.max_buffer_samples(), 48000 * 2 * 1800);
    }

    #[test]
    fn recording_config_max_buffer_high_rate() {
        let cfg = RecordingConfig {
            sample_rate: 192000,
            channels: 8,
            max_duration_secs: 600,
            ..Default::default()
        };
        assert_eq!(cfg.max_buffer_samples(), 192000 * 8 * 600);
    }

    #[test]
    fn recording_config_validated_snaps_rate() {
        let cfg = RecordingConfig {
            sample_rate: 50000, // not a supported rate
            ..Default::default()
        }
        .validated();
        assert!(RecordingConfig::SUPPORTED_RATES.contains(&cfg.sample_rate));
        // 50000 is closest to 48000
        assert_eq!(cfg.sample_rate, 48000);
    }

    #[test]
    fn recording_config_validated_clamps_channels() {
        let too_many = RecordingConfig {
            channels: 32,
            ..Default::default()
        }
        .validated();
        assert_eq!(too_many.channels, RecordingConfig::MAX_CHANNELS);

        let zero = RecordingConfig {
            channels: 0,
            ..Default::default()
        }
        .validated();
        assert_eq!(zero.channels, 1);
    }

    #[test]
    fn recording_config_validated_clamps_buffer_size() {
        let too_small = RecordingConfig {
            buffer_size: 8,
            ..Default::default()
        }
        .validated();
        assert_eq!(too_small.buffer_size, 64);

        let too_big = RecordingConfig {
            buffer_size: 99999,
            ..Default::default()
        }
        .validated();
        assert_eq!(too_big.buffer_size, 4096);
    }

    #[test]
    fn recording_config_validated_keeps_good_values() {
        let cfg = RecordingConfig {
            sample_rate: 96000,
            channels: 4,
            buffer_size: 512,
            max_duration_secs: 300,
            input_device: Some("My Mic".into()),
        }
        .validated();
        assert_eq!(cfg.sample_rate, 96000);
        assert_eq!(cfg.channels, 4);
        assert_eq!(cfg.buffer_size, 512);
        assert_eq!(cfg.max_duration_secs, 300);
        assert_eq!(cfg.input_device, Some("My Mic".into()));
    }

    #[test]
    fn recording_config_all_supported_rates_pass_validation() {
        for &rate in RecordingConfig::SUPPORTED_RATES {
            let cfg = RecordingConfig {
                sample_rate: rate,
                ..Default::default()
            }
            .validated();
            assert_eq!(cfg.sample_rate, rate);
        }
    }

    #[test]
    fn preferences_includes_recording_config() {
        let prefs = Preferences::default();
        assert_eq!(prefs.recording.sample_rate, 48000);
        assert_eq!(prefs.recording.channels, 2);
    }

    #[cfg(unix)]
    #[test]
    fn test_save_sets_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secure_prefs.json");
        let prefs = Preferences::default();
        prefs.save(&path).unwrap();

        let meta = std::fs::metadata(&path).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "preferences file should have mode 0600, got {mode:o}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_save_permissions_after_overwrite() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("overwrite_prefs.json");

        // First save
        let prefs = Preferences::default();
        prefs.save(&path).unwrap();

        // Second save (overwrite)
        let prefs2 = Preferences {
            sample_rate: 96000,
            ..Preferences::default()
        };
        prefs2.save(&path).unwrap();

        let meta = std::fs::metadata(&path).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        // Verify content was updated
        let loaded = Preferences::load(&path).unwrap();
        assert_eq!(loaded.sample_rate, 96000);
    }

    #[test]
    fn preferences_recording_config_roundtrip() {
        let prefs = Preferences {
            recording: RecordingConfig {
                sample_rate: 192000,
                channels: 8,
                max_duration_secs: 600,
                buffer_size: 1024,
                input_device: Some("USB Audio".into()),
            },
            ..Preferences::default()
        };

        let tmp = std::env::temp_dir().join("shruti_test_rec_prefs.json");
        prefs.save(&tmp).unwrap();

        let loaded = Preferences::load(&tmp).unwrap();
        assert_eq!(loaded.recording.sample_rate, 192000);
        assert_eq!(loaded.recording.channels, 8);
        assert_eq!(loaded.recording.max_duration_secs, 600);
        assert_eq!(loaded.recording.buffer_size, 1024);
        assert_eq!(loaded.recording.input_device, Some("USB Audio".into()));

        std::fs::remove_file(&tmp).ok();
    }
}
