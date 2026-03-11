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
