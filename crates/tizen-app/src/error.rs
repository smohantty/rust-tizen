use std::fmt;

/// Errors surfaced by the safe wrapper.
#[derive(Debug)]
pub enum AppError {
    /// `ui_app_main` returned a non-zero status code (off-Tizen this never fires).
    Main(i32),
    /// The user-supplied `Lifecycle::create` returned an error.
    Create(Box<dyn std::error::Error + Send + Sync + 'static>),
    /// Lifecycle callback panicked. The wrapper caught the panic so we don't
    /// unwind into C; the original payload is dropped.
    Panic,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Main(rc) => write!(f, "ui_app_main returned {rc}"),
            AppError::Create(e) => write!(f, "lifecycle create failed: {e}"),
            AppError::Panic => write!(f, "lifecycle callback panicked"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Create(e) => Some(&**e),
            _ => None,
        }
    }
}
