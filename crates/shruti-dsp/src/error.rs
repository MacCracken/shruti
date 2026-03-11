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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = AudioError::Io(io_err);
        let msg = err.to_string();
        assert!(msg.contains("I/O error"));
        assert!(msg.contains("file missing"));
    }

    #[test]
    fn display_unsupported_format() {
        let err = AudioError::UnsupportedFormat("ogg".into());
        assert_eq!(err.to_string(), "unsupported format: ogg");
    }

    #[test]
    fn display_decoding_error() {
        let err = AudioError::DecodingError("corrupt header".into());
        assert_eq!(err.to_string(), "decoding error: corrupt header");
    }

    #[test]
    fn display_export_error() {
        let err = AudioError::ExportError("disk full".into());
        assert_eq!(err.to_string(), "export error: disk full");
    }

    #[test]
    fn display_buffer_mismatch() {
        let err = AudioError::BufferMismatch {
            expected: 1024,
            got: 512,
        };
        assert_eq!(
            err.to_string(),
            "buffer size mismatch: expected 1024, got 512"
        );
    }

    #[test]
    fn source_returns_inner_for_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken");
        let err = AudioError::Io(io_err);
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn source_returns_none_for_others() {
        let err = AudioError::ExportError("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: AudioError = io_err.into();
        assert!(matches!(err, AudioError::Io(_)));
    }

    #[test]
    fn debug_impl_contains_variant_name() {
        let err = AudioError::BufferMismatch {
            expected: 100,
            got: 50,
        };
        let debug = format!("{err:?}");
        assert!(debug.contains("BufferMismatch"));
        assert!(debug.contains("100"));
        assert!(debug.contains("50"));
    }

    #[test]
    fn debug_impl_decoding_error() {
        let err = AudioError::DecodingError("bad data".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("DecodingError"));
        assert!(debug.contains("bad data"));
    }

    #[test]
    fn source_returns_none_for_unsupported_format() {
        let err = AudioError::UnsupportedFormat("aiff".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn source_returns_none_for_decoding_error() {
        let err = AudioError::DecodingError("corrupt".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn source_returns_none_for_buffer_mismatch() {
        let err = AudioError::BufferMismatch {
            expected: 256,
            got: 128,
        };
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn display_empty_strings() {
        let err = AudioError::UnsupportedFormat(String::new());
        assert_eq!(err.to_string(), "unsupported format: ");

        let err = AudioError::DecodingError(String::new());
        assert_eq!(err.to_string(), "decoding error: ");

        let err = AudioError::ExportError(String::new());
        assert_eq!(err.to_string(), "export error: ");
    }

    #[test]
    fn buffer_mismatch_zero_values() {
        let err = AudioError::BufferMismatch {
            expected: 0,
            got: 0,
        };
        assert_eq!(err.to_string(), "buffer size mismatch: expected 0, got 0");
    }
}
