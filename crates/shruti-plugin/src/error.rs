use std::fmt;

/// Errors that can occur in plugin hosting.
#[derive(Debug)]
pub enum PluginError {
    /// Plugin file not found or inaccessible.
    NotFound(String),
    /// Plugin loading or symbol resolution error.
    LoadError(String),
    /// Plugin state serialization/validation error.
    StateError(String),
    /// Scanner error.
    ScanError(String),
    /// File I/O error.
    Io(std::io::Error),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::NotFound(msg) => write!(f, "plugin not found: {msg}"),
            PluginError::LoadError(msg) => write!(f, "plugin load error: {msg}"),
            PluginError::StateError(msg) => write!(f, "plugin state error: {msg}"),
            PluginError::ScanError(msg) => write!(f, "plugin scan error: {msg}"),
            PluginError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for PluginError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PluginError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PluginError {
    fn from(e: std::io::Error) -> Self {
        PluginError::Io(e)
    }
}

impl From<String> for PluginError {
    fn from(s: String) -> Self {
        PluginError::LoadError(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_all_variants() {
        let cases: Vec<(PluginError, &str)> = vec![
            (
                PluginError::NotFound("/path/to/plugin.clap".into()),
                "plugin not found",
            ),
            (
                PluginError::LoadError("symbol not found".into()),
                "plugin load error",
            ),
            (
                PluginError::StateError("blob too large".into()),
                "plugin state error",
            ),
            (
                PluginError::ScanError("permission denied".into()),
                "plugin scan error",
            ),
            (
                PluginError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing")),
                "I/O error",
            ),
        ];

        for (err, expected_prefix) in cases {
            let msg = err.to_string();
            assert!(
                msg.contains(expected_prefix),
                "'{msg}' should contain '{expected_prefix}'"
            );
        }
    }

    #[test]
    fn display_not_found_includes_path() {
        let err = PluginError::NotFound("/usr/lib/plugin.so".into());
        assert_eq!(err.to_string(), "plugin not found: /usr/lib/plugin.so");
    }

    #[test]
    fn display_load_error_includes_detail() {
        let err = PluginError::LoadError("clap_entry not found".into());
        assert_eq!(err.to_string(), "plugin load error: clap_entry not found");
    }

    #[test]
    fn display_state_error_includes_detail() {
        let err = PluginError::StateError("blob too large: 20MB".into());
        assert_eq!(err.to_string(), "plugin state error: blob too large: 20MB");
    }

    #[test]
    fn display_scan_error_includes_detail() {
        let err = PluginError::ScanError("directory not readable".into());
        assert_eq!(err.to_string(), "plugin scan error: directory not readable");
    }

    #[test]
    fn source_returns_inner_for_io() {
        let err = PluginError::Io(std::io::Error::other("x"));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn source_returns_none_for_not_found() {
        let err = PluginError::NotFound("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn source_returns_none_for_load_error() {
        let err = PluginError::LoadError("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn source_returns_none_for_state_error() {
        let err = PluginError::StateError("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn source_returns_none_for_scan_error() {
        let err = PluginError::ScanError("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn from_io_error() {
        let err: PluginError =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied").into();
        assert!(matches!(err, PluginError::Io(_)));
    }

    #[test]
    fn from_string() {
        let err: PluginError = "something failed".to_string().into();
        assert!(matches!(err, PluginError::LoadError(_)));
        assert!(err.to_string().contains("something failed"));
    }

    #[test]
    fn debug_impl() {
        let err = PluginError::NotFound("test.clap".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("NotFound"));
        assert!(debug.contains("test.clap"));
    }
}
