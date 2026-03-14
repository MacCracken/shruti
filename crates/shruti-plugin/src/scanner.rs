use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::error::PluginError;
use crate::format::PluginFormat;

/// A plugin discovered during scanning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedPlugin {
    pub path: PathBuf,
    pub format: PluginFormat,
    pub name: String,
}

/// Maximum depth for following symlinks during directory traversal.
/// Prevents infinite loops from circular symlinks.
pub const MAX_SYMLINK_DEPTH: usize = 5;

/// Cached scan results, persisted alongside scan results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCache {
    /// Map from directory path to its modification time (as seconds since epoch)
    /// and the plugins found in that directory.
    pub entries: HashMap<PathBuf, ScanCacheEntry>,
}

/// A single cache entry for a scanned directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCacheEntry {
    /// Directory modification time as seconds since UNIX epoch.
    pub mtime_secs: u64,
    /// Plugins discovered in this directory.
    pub plugins: Vec<ScannedPlugin>,
}

impl ScanCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Load cache from a JSON file. Returns empty cache if file doesn't exist or is invalid.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    /// Save cache to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), PluginError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| PluginError::ScanError(e.to_string()))?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Check if a directory's cached results are still valid (modification time unchanged).
    pub fn is_valid(&self, dir: &Path) -> bool {
        if let Some(entry) = self.entries.get(dir)
            && let Ok(meta) = std::fs::metadata(dir)
            && let Ok(mtime) = meta.modified()
        {
            let secs = mtime
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return secs == entry.mtime_secs;
        }
        false
    }

    /// Get cached plugins for a directory, if the cache is still valid.
    pub fn get(&self, dir: &Path) -> Option<&[ScannedPlugin]> {
        if self.is_valid(dir) {
            self.entries.get(dir).map(|e| e.plugins.as_slice())
        } else {
            None
        }
    }

    /// Update cache entry for a directory.
    pub fn update(&mut self, dir: &Path, plugins: Vec<ScannedPlugin>) {
        let mtime_secs = std::fs::metadata(dir)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.entries.insert(
            dir.to_owned(),
            ScanCacheEntry {
                mtime_secs,
                plugins,
            },
        );
    }
}

impl Default for ScanCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Scans the filesystem for installed plugins.
pub struct PluginScanner {
    /// Additional search paths beyond the standard ones.
    extra_paths: Vec<PathBuf>,
    /// Optional cache for scan results.
    cache: Option<ScanCache>,
    /// Path to the cache file on disk.
    cache_path: Option<PathBuf>,
}

impl PluginScanner {
    pub fn new() -> Self {
        Self {
            extra_paths: Vec::new(),
            cache: None,
            cache_path: None,
        }
    }

    /// Enable disk caching of scan results. The cache file will be read on
    /// the next scan and written after scanning completes.
    pub fn with_cache(mut self, cache_path: PathBuf) -> Self {
        self.cache = Some(ScanCache::load(&cache_path));
        self.cache_path = Some(cache_path);
        self
    }

    pub fn add_path(&mut self, path: impl Into<PathBuf>) {
        self.extra_paths.push(path.into());
    }

    /// Scan all known paths for plugins of all supported formats.
    ///
    /// If caching is enabled, directories whose modification time has not changed
    /// since the last scan will return cached results instead of re-scanning.
    pub fn scan_all(&mut self) -> Vec<ScannedPlugin> {
        let mut results = Vec::new();

        for format in [PluginFormat::Clap, PluginFormat::Vst3, PluginFormat::Native] {
            results.extend(self.scan_format(format));
        }

        // Persist cache if enabled
        if let (Some(cache), Some(path)) = (&self.cache, &self.cache_path) {
            let _ = cache.save(path);
        }

        results
    }

    /// Scan for plugins of a specific format.
    pub fn scan_format(&mut self, format: PluginFormat) -> Vec<ScannedPlugin> {
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

            // Check cache first
            if let Some(cache) = &self.cache
                && let Some(cached) = cache.get(dir)
            {
                // Filter cached results for this format
                let format_results: Vec<ScannedPlugin> = cached
                    .iter()
                    .filter(|p| p.format == format)
                    .cloned()
                    .collect();
                results.extend(format_results);
                continue;
            }

            let mut dir_results = Vec::new();

            // follow_links(true) enables following symlinks. WalkDir detects
            // symlink loops internally (via inode tracking) and emits errors
            // for them, which flatten() skips. MAX_SYMLINK_DEPTH limits the
            // overall traversal depth as an additional safety measure.
            for entry in WalkDir::new(dir)
                .max_depth(MAX_SYMLINK_DEPTH)
                .follow_links(true)
                .into_iter()
                .flatten()
            {
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

                    dir_results.push(ScannedPlugin {
                        path: path.to_owned(),
                        format,
                        name,
                    });
                }
            }

            // Update cache with results for this directory
            if let Some(cache) = &mut self.cache {
                cache.update(dir, dir_results.clone());
            }

            results.extend(dir_results);
        }

        // Persist the cache to disk if a path is configured
        if let (Some(cache), Some(path)) = (&self.cache, &self.cache_path) {
            let _ = cache.save(path);
        }

        results
    }
}

impl Default for PluginScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Default cache file name, placed in the system cache directory.
pub fn default_cache_path() -> PathBuf {
    let cache_dir = if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache")
    } else {
        PathBuf::from(".")
    };
    cache_dir.join("shruti").join("plugin_scan_cache.json")
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
        let mut scanner = PluginScanner::new();
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

    // --- Symlink depth tests ---

    #[test]
    fn test_scanner_symlink_depth_limit() {
        // Create a directory with a symlink loop
        let tmp = std::env::temp_dir().join("shruti_test_symlink");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // Create a symlink loop: dir_a/link -> dir_a (circular)
        let dir_a = tmp.join("dir_a");
        std::fs::create_dir_all(&dir_a).unwrap();

        #[cfg(unix)]
        {
            let link = dir_a.join("loop_link");
            let _ = std::os::unix::fs::symlink(&dir_a, &link);
        }

        // Create a real plugin so scanner has something to find
        let fake_plugin = dir_a.join("test-plugin.clap");
        std::fs::write(&fake_plugin, b"fake").unwrap();

        let mut scanner = PluginScanner::new();
        scanner.add_path(&tmp);

        // Should not hang due to symlink loop
        let results = scanner.scan_format(PluginFormat::Clap);
        assert!(results.iter().any(|p| p.name == "test-plugin"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_max_symlink_depth_is_reasonable() {
        // Verify the constant is in a reasonable range
        const { assert!(MAX_SYMLINK_DEPTH >= 3) };
        const { assert!(MAX_SYMLINK_DEPTH <= 10) };
    }

    // --- Scan cache tests ---

    #[test]
    fn test_scan_cache_empty() {
        let cache = ScanCache::new();
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_scan_cache_roundtrip() {
        let tmp = std::env::temp_dir().join("shruti_test_cache");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let cache_file = tmp.join("cache.json");

        let mut cache = ScanCache::new();
        let plugins = vec![ScannedPlugin {
            path: PathBuf::from("/fake/plugin.clap"),
            format: PluginFormat::Clap,
            name: "FakePlugin".into(),
        }];
        cache.update(&tmp, plugins);
        cache.save(&cache_file).unwrap();

        let loaded = ScanCache::load(&cache_file);
        assert!(loaded.entries.contains_key(tmp.as_path()));
        let entry = &loaded.entries[tmp.as_path()];
        assert_eq!(entry.plugins.len(), 1);
        assert_eq!(entry.plugins[0].name, "FakePlugin");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_scan_cache_validity() {
        let tmp = std::env::temp_dir().join("shruti_test_cache_valid");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let mut cache = ScanCache::new();
        cache.update(&tmp, vec![]);

        // Cache should be valid for unchanged directory
        assert!(cache.is_valid(&tmp));

        // Non-existent directory should be invalid
        assert!(!cache.is_valid(Path::new("/nonexistent/dir")));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_scan_cache_load_missing_file() {
        let cache = ScanCache::load(Path::new("/nonexistent/cache.json"));
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_scanner_with_cache() {
        let tmp = std::env::temp_dir().join("shruti_test_scanner_cache");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let cache_file = tmp.join("scan_cache.json");
        let plugin_dir = tmp.join("plugins");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        // Create a fake plugin
        let fake_plugin = plugin_dir.join("cached-reverb.clap");
        std::fs::write(&fake_plugin, b"fake").unwrap();

        // First scan: no cache
        let mut scanner = PluginScanner::new().with_cache(cache_file.clone());
        scanner.add_path(&plugin_dir);
        let results = scanner.scan_format(PluginFormat::Clap);
        assert!(results.iter().any(|p| p.name == "cached-reverb"));

        // Verify cache file was written
        assert!(cache_file.exists());

        // Second scan: should use cache
        let mut scanner2 = PluginScanner::new().with_cache(cache_file.clone());
        scanner2.add_path(&plugin_dir);
        let results2 = scanner2.scan_format(PluginFormat::Clap);
        assert!(results2.iter().any(|p| p.name == "cached-reverb"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_default_cache_path() {
        let path = default_cache_path();
        assert!(path.ends_with("shruti/plugin_scan_cache.json"));
    }
}
