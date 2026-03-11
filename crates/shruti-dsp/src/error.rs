use std::fmt;

/// Errors that can occur in audio processing.
#[derive(Debug)]
pub enum AudioError {
    /// File I/O error.
    Io(std::io::Error),
    /// Audio format not supported.
    UnsupportedFormat(String),
    /// Audio decoding failed.
    DecodingError(String),
    /// Export error.
    ExportError(String),
    /// Buffer size mismatch.
    BufferMismatch { expected: usize, got: usize },
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::Io(e) => write!(f, "I/O error: {e}"),
            AudioError::UnsupportedFormat(fmt_name) => write!(f, "unsupported format: {fmt_name}"),
            AudioError::DecodingError(msg) => write!(f, "decoding error: {msg}"),
            AudioError::ExportError(msg) => write!(f, "export error: {msg}"),
            AudioError::BufferMismatch { expected, got } => {
                write!(f, "buffer size mismatch: expected {expected}, got {got}")
            }
        }
    }
}

impl std::error::Error for AudioError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AudioError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AudioError {
    fn from(e: std::io::Error) -> Self {
        AudioError::Io(e)
    }
}
