use std::fmt;

/// Result alias for fallible operations on an [`InputGenerator`](crate::InputGenerator).
pub type Result<T> = std::result::Result<T, Error>;

/// Failure modes that this crate's public API can return.
#[derive(Debug)]
pub enum Error {
    /// `wl_display_connect()` (or its Rust equivalent) failed — usually because
    /// `$WAYLAND_DISPLAY` is unset or the socket is unreachable.
    NotConnected(String),

    /// The compositor does not advertise a `tizen_input_device_manager` global.
    /// Stock Wayland compositors (Weston/Mutter/KWin/Sway) do not — this
    /// crate only works against a Tizen compositor (Enlightenment E20).
    ProtocolUnavailable,

    /// The compositor rejected the request. Mirrors the protocol's `error`
    /// event codes; the underlying `u32` is preserved for callers who want
    /// to inspect novel codes from a newer protocol version.
    Rejected(ProtocolError),

    /// A roundtrip on the Wayland event queue failed mid-operation.
    Transport(String),

    /// The crate was called on a non-Tizen host build. Host builds compile
    /// and link but can only inject events if a Tizen-protocol-speaking
    /// compositor is reachable via `$WAYLAND_DISPLAY`.
    Unsupported,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected(msg) => write!(f, "wayland connect failed: {msg}"),
            Self::ProtocolUnavailable => f.write_str(
                "compositor does not advertise tizen_input_device_manager — \
                 this only works on a Tizen compositor",
            ),
            Self::Rejected(err) => write!(f, "compositor rejected the request: {err}"),
            Self::Transport(msg) => write!(f, "wayland transport error: {msg}"),
            Self::Unsupported => f.write_str(
                "tizen-input not supported on this host build (no reachable Tizen compositor)",
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Mirrors the `error` enum on the `tizen_input_device_manager` interface.
///
/// `Unknown(_)` preserves codes that newer protocol revisions might add
/// without forcing a crate bump.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProtocolError {
    /// `no_permission` (1) — the caller lacks the platform privilege needed
    /// to inject events.
    NoPermission,
    /// `invalid_class` (2) — the device class bitmask is not supported.
    InvalidClass,
    /// `blocked_already` (3) — another client has already taken this
    /// generator class.
    BlockedAlready,
    /// `no_system_resources` (4) — out of memory / resources server-side.
    NoSystemResources,
    /// `invalid_parameter` (5) — bad argument (e.g. unknown key name).
    InvalidParameter,
    /// `invalid_surface` (6) — surface argument is not visible or pointer
    /// is not over it.
    InvalidSurface,
    /// `no_pointer_available` (7) — no pointer device to warp.
    NoPointerAvailable,
    /// `not_allowed` (8) — system policy forbids this request.
    NotAllowed,
    /// Forward-compatibility hatch for codes added in future protocol versions.
    Unknown(u32),
}

impl ProtocolError {
    pub(crate) fn from_code(code: u32) -> Self {
        match code {
            1 => Self::NoPermission,
            2 => Self::InvalidClass,
            3 => Self::BlockedAlready,
            4 => Self::NoSystemResources,
            5 => Self::InvalidParameter,
            6 => Self::InvalidSurface,
            7 => Self::NoPointerAvailable,
            8 => Self::NotAllowed,
            other => Self::Unknown(other),
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPermission => f.write_str("no_permission"),
            Self::InvalidClass => f.write_str("invalid_class"),
            Self::BlockedAlready => f.write_str("blocked_already"),
            Self::NoSystemResources => f.write_str("no_system_resources"),
            Self::InvalidParameter => f.write_str("invalid_parameter"),
            Self::InvalidSurface => f.write_str("invalid_surface"),
            Self::NoPointerAvailable => f.write_str("no_pointer_available"),
            Self::NotAllowed => f.write_str("not_allowed"),
            Self::Unknown(code) => write!(f, "unknown({code})"),
        }
    }
}
