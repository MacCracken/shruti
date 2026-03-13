use std::path::Path;

use super::colors::ThemeColors;

/// A complete theme definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Shruti Dark".into(),
            colors: ThemeColors::default(),
        }
    }
}

impl Theme {
    /// Load a theme from a JSON file.
    ///
    /// Returns a human-readable error message if the file cannot be read,
    /// contains malformed JSON, or is missing required fields.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read theme: {e}"))?;
        let theme: Self =
            serde_json::from_str(&content).map_err(|e| format!("Failed to parse theme: {e}"))?;
        theme.validate()?;
        Ok(theme)
    }

    /// Validate theme fields for sanity.
    ///
    /// Returns `Ok(())` if valid, or an error message describing the problem.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Theme name must not be empty".to_string());
        }
        // All color arrays must have alpha > 0 for at least the primary colors
        // (completely transparent primary colors are likely a mistake)
        let critical_colors = [
            ("bg_primary", self.colors.bg_primary),
            ("text_primary", self.colors.text_primary),
            ("accent", self.colors.accent),
        ];
        for (name, rgba) in &critical_colors {
            if rgba[3] == 0 {
                return Err(format!(
                    "Theme color '{name}' has zero alpha; this would make it invisible"
                ));
            }
        }
        Ok(())
    }

    /// Save the current theme to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize theme: {e}"))?;
        std::fs::write(path, content).map_err(|e| format!("Failed to write theme: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn default_theme_has_expected_name() {
        let theme = Theme::default();
        assert_eq!(theme.name, "Shruti Dark");
    }

    #[test]
    fn default_theme_has_valid_colors() {
        let theme = Theme::default();
        // Just verify colors are the default set
        assert_eq!(theme.colors.bg_primary, [24, 24, 28, 255]);
        assert_eq!(theme.colors.accent, [60, 130, 240, 255]);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("shruti_test_theme");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("test_theme.json");

        let original = Theme::default();
        original.save(&path).expect("save should succeed");

        let loaded = Theme::load(&path).expect("load should succeed");
        assert_eq!(loaded.name, original.name);
        assert_eq!(loaded.colors.bg_primary, original.colors.bg_primary);
        assert_eq!(loaded.colors.accent, original.colors.accent);
        assert_eq!(loaded.colors.waveform, original.colors.waveform);

        // Clean up
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn load_nonexistent_file_returns_error() {
        let path = PathBuf::from("/tmp/shruti_nonexistent_theme_file_12345.json");
        let result = Theme::load(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read theme"));
    }

    #[test]
    fn load_invalid_json_returns_error() {
        let dir = std::env::temp_dir().join("shruti_test_theme_invalid");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("bad_theme.json");
        std::fs::write(&path, "{ not valid json }").ok();

        let result = Theme::load(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse theme"));

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn validate_default_theme_passes() {
        let theme = Theme::default();
        assert!(theme.validate().is_ok());
    }

    #[test]
    fn validate_empty_name_fails() {
        let mut theme = Theme::default();
        theme.name = "".into();
        let result = theme.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("name must not be empty"));
    }

    #[test]
    fn validate_whitespace_only_name_fails() {
        let mut theme = Theme::default();
        theme.name = "   ".into();
        let result = theme.validate();
        assert!(result.is_err());
    }

    #[test]
    fn validate_zero_alpha_primary_color_fails() {
        let mut theme = Theme::default();
        theme.colors.bg_primary = [24, 24, 28, 0]; // zero alpha
        let result = theme.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("bg_primary"));
    }

    #[test]
    fn validate_zero_alpha_accent_fails() {
        let mut theme = Theme::default();
        theme.colors.accent = [60, 130, 240, 0];
        let result = theme.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("accent"));
    }

    #[test]
    fn validate_zero_alpha_text_primary_fails() {
        let mut theme = Theme::default();
        theme.colors.text_primary = [220, 220, 225, 0];
        let result = theme.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("text_primary"));
    }

    #[test]
    fn load_malformed_missing_fields_returns_error() {
        let dir = std::env::temp_dir().join("shruti_test_theme_partial");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("partial_theme.json");
        // Valid JSON but missing required fields
        std::fs::write(&path, r#"{"name": "Incomplete"}"#).ok();

        let result = Theme::load(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse theme"));

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }
}
