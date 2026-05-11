use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Main(i32),
    Create(Box<dyn std::error::Error + Send + Sync + 'static>),
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
