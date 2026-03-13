use std::fmt;

/// Errors that can occur in session management.
#[derive(Debug)]
pub enum SessionError {
    /// File I/O error.
    Io(std::io::Error),
    /// Database error.
    Database(String),
    /// Serialization/deserialization error.
    Serialization(String),
    /// Track not found.
    TrackNotFound(String),
    /// Region not found.
    RegionNotFound(String),
    /// Invalid operation.
    InvalidOperation(String),
    /// Audio processing error.
    Audio(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::Io(e) => write!(f, "I/O error: {e}"),
            SessionError::Database(msg) => write!(f, "database error: {msg}"),
            SessionError::Serialization(msg) => write!(f, "serialization error: {msg}"),
            SessionError::TrackNotFound(name) => write!(f, "track not found: {name}"),
            SessionError::RegionNotFound(id) => write!(f, "region not found: {id}"),
            SessionError::InvalidOperation(msg) => write!(f, "invalid operation: {msg}"),
            SessionError::Audio(msg) => write!(f, "audio error: {msg}"),
        }
    }
}

impl std::error::Error for SessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SessionError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SessionError {
    fn from(e: std::io::Error) -> Self {
        SessionError::Io(e)
    }
}

impl From<rusqlite::Error> for SessionError {
    fn from(e: rusqlite::Error) -> Self {
        SessionError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for SessionError {
    fn from(e: serde_json::Error) -> Self {
        SessionError::Serialization(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_all_variants() {
        let cases: Vec<(SessionError, &str)> = vec![
            (
                SessionError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone")),
                "I/O error",
            ),
            (SessionError::Database("corrupt".into()), "database error"),
            (
                SessionError::Serialization("bad json".into()),
                "serialization error",
            ),
            (
                SessionError::TrackNotFound("track_1".into()),
                "track not found",
            ),
            (
                SessionError::RegionNotFound("region_42".into()),
                "region not found",
            ),
            (
                SessionError::InvalidOperation("nope".into()),
                "invalid operation",
            ),
            (SessionError::Audio("clipping".into()), "audio error"),
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
    fn source_returns_inner_for_io() {
        let err = SessionError::Io(std::io::Error::other("x"));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn source_returns_none_for_others() {
        let err = SessionError::Database("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn from_io_error() {
        let err: SessionError =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied").into();
        assert!(matches!(err, SessionError::Io(_)));
    }

    #[test]
    fn from_rusqlite_error() {
        let err: SessionError = rusqlite::Error::InvalidQuery.into();
        assert!(matches!(err, SessionError::Database(_)));
    }

    #[test]
    fn from_serde_json_error() {
        let json_err = serde_json::from_str::<String>("not valid json{{{").unwrap_err();
        let err: SessionError = json_err.into();
        assert!(matches!(err, SessionError::Serialization(_)));
    }
}
