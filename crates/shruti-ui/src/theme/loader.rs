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
    pub fn load(path: &Path) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read theme: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse theme: {e}"))
    }

    /// Save the current theme to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize theme: {e}"))?;
        std::fs::write(path, content).map_err(|e| format!("Failed to write theme: {e}"))
    }
}
