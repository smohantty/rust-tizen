use std::fmt;

/// Result alias for fallible operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Failure modes for the window stack.
#[derive(Debug)]
pub enum Error {
    /// Wayland connect failed (no `$WAYLAND_DISPLAY` set or socket
    /// unreachable).
    NotConnected(String),

    /// Compositor doesn't advertise a required global. The argument
    /// is the missing interface name.
    GlobalMissing(&'static str),

    /// TBM client init / buffer allocation failed.
    Tbm(String),

    /// Wayland event-queue transport error.
    Transport(String),

    /// Crate was called on a non-Tizen build with no reachable
    /// Tizen-compatible compositor.
    Unsupported,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected(m) => write!(f, "wayland connect failed: {m}"),
            Self::GlobalMissing(name) => write!(
                f,
                "compositor does not advertise `{name}` — needed for native Tizen window"
            ),
            Self::Tbm(m) => write!(f, "tbm: {m}"),
            Self::Transport(m) => write!(f, "wayland transport: {m}"),
            Self::Unsupported => f.write_str("tizen-window not supported on this host build"),
        }
    }
}

impl std::error::Error for Error {}
