use serde::{Deserialize, Serialize};

/// Supported plugin formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginFormat {
    /// CLAP (CLever Audio Plugin) — modern, open standard.
    Clap,
    /// VST3 — Steinberg's plugin format.
    Vst3,
    /// Native Rust plugin — Shruti's built-in plugin API.
    Native,
}

impl PluginFormat {
    /// File extension for this format's plugin bundles.
    pub fn extension(&self) -> &str {
        match self {
            Self::Clap => "clap",
            Self::Vst3 => "vst3",
            Self::Native => "so", // .so on Linux, .dylib on macOS, .dll on Windows
        }
    }

    /// Standard search paths for this format on the current platform.
    pub fn search_paths(&self) -> Vec<String> {
        match self {
            Self::Clap => clap_search_paths(),
            Self::Vst3 => vst3_search_paths(),
            Self::Native => native_search_paths(),
        }
    }
}

fn clap_search_paths() -> Vec<String> {
    let mut paths = Vec::new();

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{home}/.clap"));
        }
        paths.push("/usr/lib/clap".into());
        paths.push("/usr/local/lib/clap".into());
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{home}/Library/Audio/Plug-Ins/CLAP"));
        }
        paths.push("/Library/Audio/Plug-Ins/CLAP".into());
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(common) = std::env::var("CommonProgramFiles") {
            paths.push(format!("{common}\\CLAP"));
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            paths.push(format!("{local}\\Programs\\Common\\CLAP"));
        }
    }

    paths
}

fn vst3_search_paths() -> Vec<String> {
    let mut paths = Vec::new();

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{home}/.vst3"));
        }
        paths.push("/usr/lib/vst3".into());
        paths.push("/usr/local/lib/vst3".into());
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{home}/Library/Audio/Plug-Ins/VST3"));
        }
        paths.push("/Library/Audio/Plug-Ins/VST3".into());
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(common) = std::env::var("CommonProgramFiles") {
            paths.push(format!("{common}\\VST3"));
        }
    }

    paths
}

fn native_search_paths() -> Vec<String> {
    let mut paths = Vec::new();

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{home}/.shruti/plugins"));
        }
        paths.push("/usr/lib/shruti/plugins".into());
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{home}/Library/Audio/Plug-Ins/Shruti"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            paths.push(format!("{local}\\Shruti\\plugins"));
        }
    }

    paths
}
