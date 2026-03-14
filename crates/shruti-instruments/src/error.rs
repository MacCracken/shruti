use std::fmt;

/// Errors that can occur in instrument loading and configuration.
#[derive(Debug)]
pub enum InstrumentError {
    /// File format parsing error (SFZ/SF2).
    ParseError(String),
    /// Invalid parameter or configuration.
    InvalidConfig(String),
    /// File I/O error.
    Io(std::io::Error),
}

impl fmt::Display for InstrumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstrumentError::ParseError(msg) => write!(f, "parse error: {msg}"),
            InstrumentError::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
            InstrumentError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for InstrumentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InstrumentError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for InstrumentError {
    fn from(e: std::io::Error) -> Self {
        InstrumentError::Io(e)
    }
}

impl From<String> for InstrumentError {
    fn from(s: String) -> Self {
        InstrumentError::ParseError(s)
    }
}

impl From<&str> for InstrumentError {
    fn from(s: &str) -> Self {
        InstrumentError::ParseError(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_all_variants() {
        let cases: Vec<(InstrumentError, &str)> = vec![
            (
                InstrumentError::ParseError("unexpected end of data".into()),
                "parse error",
            ),
            (
                InstrumentError::InvalidConfig("sample rate must be > 0".into()),
                "invalid config",
            ),
            (
                InstrumentError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing")),
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
    fn display_parse_error_includes_detail() {
        let err = InstrumentError::ParseError("not a RIFF file".into());
        assert_eq!(err.to_string(), "parse error: not a RIFF file");
    }

    #[test]
    fn display_invalid_config_includes_detail() {
        let err = InstrumentError::InvalidConfig("negative sample rate".into());
        assert_eq!(err.to_string(), "invalid config: negative sample rate");
    }

    #[test]
    fn source_returns_inner_for_io() {
        let err = InstrumentError::Io(std::io::Error::other("x"));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn source_returns_none_for_parse_error() {
        let err = InstrumentError::ParseError("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn source_returns_none_for_invalid_config() {
        let err = InstrumentError::InvalidConfig("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn from_io_error() {
        let err: InstrumentError =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied").into();
        assert!(matches!(err, InstrumentError::Io(_)));
    }

    #[test]
    fn from_string() {
        let err: InstrumentError = "bad format".to_string().into();
        assert!(matches!(err, InstrumentError::ParseError(_)));
        assert!(err.to_string().contains("bad format"));
    }

    #[test]
    fn debug_impl() {
        let err = InstrumentError::ParseError("corrupt header".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("ParseError"));
        assert!(debug.contains("corrupt header"));
    }
}
