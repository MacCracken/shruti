use std::fmt;

/// Errors that can occur in the audio engine.
#[derive(Debug)]
pub enum EngineError {
    /// Audio backend initialization or device error.
    Backend(String),
    /// Graph compilation error.
    Graph(String),
    /// Recording error.
    Recording(String),
    /// File I/O error.
    Io(std::io::Error),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::Backend(msg) => write!(f, "backend error: {msg}"),
            EngineError::Graph(msg) => write!(f, "graph error: {msg}"),
            EngineError::Recording(msg) => write!(f, "recording error: {msg}"),
            EngineError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for EngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EngineError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for EngineError {
    fn from(e: std::io::Error) -> Self {
        EngineError::Io(e)
    }
}

impl From<Box<dyn std::error::Error>> for EngineError {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        EngineError::Backend(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_all_variants() {
        let cases: Vec<(EngineError, &str)> = vec![
            (EngineError::Backend("no device".into()), "backend error"),
            (EngineError::Graph("cycle detected".into()), "graph error"),
            (
                EngineError::Recording("thread panicked".into()),
                "recording error",
            ),
            (
                EngineError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing")),
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
    fn display_backend_includes_message() {
        let err = EngineError::Backend("ALSA device not found".into());
        assert_eq!(err.to_string(), "backend error: ALSA device not found");
    }

    #[test]
    fn display_graph_includes_message() {
        let err = EngineError::Graph("cycle detected".into());
        assert_eq!(err.to_string(), "graph error: cycle detected");
    }

    #[test]
    fn display_recording_includes_message() {
        let err = EngineError::Recording("disk full".into());
        assert_eq!(err.to_string(), "recording error: disk full");
    }

    #[test]
    fn source_returns_inner_for_io() {
        let err = EngineError::Io(std::io::Error::other("x"));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn source_returns_none_for_backend() {
        let err = EngineError::Backend("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn source_returns_none_for_graph() {
        let err = EngineError::Graph("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn source_returns_none_for_recording() {
        let err = EngineError::Recording("x".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn from_io_error() {
        let err: EngineError =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied").into();
        assert!(matches!(err, EngineError::Io(_)));
    }

    #[test]
    fn from_boxed_error() {
        let boxed: Box<dyn std::error::Error> = "something went wrong".into();
        let err: EngineError = boxed.into();
        assert!(matches!(err, EngineError::Backend(_)));
        assert!(err.to_string().contains("something went wrong"));
    }

    #[test]
    fn debug_impl() {
        let err = EngineError::Backend("test".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("Backend"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn from_io_error_preserves_kind() {
        let io_err = std::io::Error::new(std::io::ErrorKind::WouldBlock, "would block");
        let err: EngineError = io_err.into();
        match &err {
            EngineError::Io(inner) => {
                assert_eq!(inner.kind(), std::io::ErrorKind::WouldBlock);
            }
            _ => panic!("expected Io variant"),
        }
    }

    #[test]
    fn display_io_includes_inner_message() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file gone");
        let err = EngineError::Io(io_err);
        let msg = err.to_string();
        assert!(msg.contains("file gone"), "got: {msg}");
    }

    #[test]
    fn from_boxed_io_error() {
        let io_err: Box<dyn std::error::Error> =
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, "io boxed"));
        let err: EngineError = io_err.into();
        assert!(matches!(err, EngineError::Backend(_)));
        assert!(err.to_string().contains("io boxed"));
    }

    #[test]
    fn error_trait_is_implemented() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<EngineError>();
    }

    #[test]
    fn source_io_chain() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "inner cause");
        let err = EngineError::Io(io_err);
        let source = std::error::Error::source(&err).unwrap();
        assert!(source.to_string().contains("inner cause"));
    }

    #[test]
    fn debug_all_variants() {
        let cases: Vec<EngineError> = vec![
            EngineError::Backend("b".into()),
            EngineError::Graph("g".into()),
            EngineError::Recording("r".into()),
            EngineError::Io(std::io::Error::other("i")),
        ];
        for err in &cases {
            let debug = format!("{err:?}");
            assert!(!debug.is_empty());
        }
    }
}
