use std::fmt;

/// Result alias for fallible operations on a
/// [`ScreenCapturer`](crate::ScreenCapturer).
pub type Result<T> = std::result::Result<T, Error>;

/// Failure modes that this crate's public API can return.
#[derive(Debug)]
pub enum Error {
    /// `wl_display_connect()` (or its Rust equivalent) failed — usually
    /// because `$WAYLAND_DISPLAY` is unset/unreachable.
    NotConnected(String),

    /// The compositor does not advertise `tizen_screenshooter`. Stock
    /// Wayland compositors (Weston/Mutter/KWin/Sway) do not — this
    /// crate only works against a Tizen compositor.
    ProtocolUnavailable,

    /// The compositor advertised `tizen_screenshooter` but the version
    /// is too old. We require version 3+ for synchronous `shoot`.
    ProtocolTooOld {
        /// Maximum interface version the compositor offered.
        advertised: u32,
        /// Minimum version this crate needs.
        required: u32,
    },

    /// The compositor did not advertise a `wl_output`. Without one we
    /// have nothing to screenshot from.
    NoOutput,

    /// `libwayland-tbm-client.so.0` could not be loaded at runtime.
    /// The .so should be present on every Tizen device — its absence
    /// usually means the test target isn't a Tizen system.
    TbmUnavailable(String),

    /// `tbm_surface_create` or `wayland_tbm_client_create_buffer` failed.
    BufferAllocation(String),

    /// `tbm_surface_map` failed; the buffer contains pixels but we
    /// couldn't address them from CPU memory.
    BufferMap(String),

    /// A roundtrip on the Wayland event queue failed.
    Transport(String),

    /// The crate was called on a non-Tizen host build. The capture
    /// path depends on the Tizen compositor + TBM, which have no
    /// reasonable host fallback.
    Unsupported,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected(m) => write!(f, "wayland connect failed: {m}"),
            Self::ProtocolUnavailable => f.write_str(
                "compositor does not advertise tizen_screenshooter — \
                 this only works on a Tizen compositor",
            ),
            Self::ProtocolTooOld {
                advertised,
                required,
            } => write!(
                f,
                "tizen_screenshooter version {advertised} on the compositor; \
                 this crate needs at least {required}"
            ),
            Self::NoOutput => f.write_str("no wl_output advertised — nothing to capture"),
            Self::TbmUnavailable(m) => write!(f, "tbm client init failed: {m}"),
            Self::BufferAllocation(m) => write!(f, "tbm buffer allocation failed: {m}"),
            Self::BufferMap(m) => write!(f, "tbm buffer map failed: {m}"),
            Self::Transport(m) => write!(f, "wayland transport error: {m}"),
            Self::Unsupported => {
                f.write_str("tizen-screenshot not supported on this host build (Tizen-only)")
            }
        }
    }
}

impl std::error::Error for Error {}
