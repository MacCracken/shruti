use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::format::PluginFormat;

/// A plugin discovered during scanning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedPlugin {
    pub path: PathBuf,
    pub format: PluginFormat,
    pub name: String,
}

/// Scans the filesystem for installed plugins.
pub struct PluginScanner {
    /// Additional search paths beyond the standard ones.
    extra_paths: Vec<PathBuf>,
}

impl PluginScanner {
    pub fn new() -> Self {
        Self {
            extra_paths: Vec::new(),
        }
    }

    pub fn add_path(&mut self, path: impl Into<PathBuf>) {
        self.extra_paths.push(path.into());
    }

    /// Scan all known paths for plugins of all supported formats.
    pub fn scan_all(&self) -> Vec<ScannedPlugin> {
        let mut results = Vec::new();

        for format in [PluginFormat::Clap, PluginFormat::Vst3, PluginFormat::Native] {
            results.extend(self.scan_format(format));
        }

        results
    }

    /// Scan for plugins of a specific format.
    pub fn scan_format(&self, format: PluginFormat) -> Vec<ScannedPlugin> {
        let mut results = Vec::new();
        let ext = format.extension();

        let mut search_paths: Vec<PathBuf> = format
            .search_paths()
            .into_iter()
            .map(PathBuf::from)
            .collect();
        search_paths.extend(self.extra_paths.clone());

        for dir in &search_paths {
            if !dir.exists() {
                continue;
            }

            for entry in WalkDir::new(dir).max_depth(3).into_iter().flatten() {
                let path = entry.path();

                let matches = match format {
                    PluginFormat::Vst3 => {
                        // VST3 bundles are directories ending in .vst3
                        path.is_dir()
                            && path
                                .extension()
                                .is_some_and(|e| e.eq_ignore_ascii_case(ext))
                    }
                    PluginFormat::Clap => {
                        // CLAP plugins are shared libraries ending in .clap
                        path.is_file()
                            && path
                                .extension()
                                .is_some_and(|e| e.eq_ignore_ascii_case(ext))
                    }
                    PluginFormat::Native => {
                        // Native plugins are shared libraries
                        path.is_file() && is_shared_lib(path)
                    }
                };

                if matches {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    results.push(ScannedPlugin {
                        path: path.to_owned(),
                        format,
                        name,
                    });
                }
            }
        }

        results
    }
}

impl Default for PluginScanner {
    fn default() -> Self {
        Self::new()
    }
}

fn is_shared_lib(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        let ext = ext.to_string_lossy().to_lowercase();
        ext == "so" || ext == "dylib" || ext == "dll"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_empty_paths() {
        let scanner = PluginScanner::new();
        // Should not panic on non-existent paths
        let results = scanner.scan_all();
        // Results depend on system — just verify no crash
        let _ = results;
    }

    #[test]
    fn test_scanner_custom_path() {
        let tmp = std::env::temp_dir().join("shruti_test_plugins");
        let _ = std::fs::create_dir_all(&tmp);

        // Create a fake .clap file
        let fake_plugin = tmp.join("test-reverb.clap");
        std::fs::write(&fake_plugin, b"fake").unwrap();

        let mut scanner = PluginScanner::new();
        scanner.add_path(&tmp);

        let results = scanner.scan_format(PluginFormat::Clap);
        assert!(results.iter().any(|p| p.name == "test-reverb"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
