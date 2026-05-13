//! Scanner-generated raw client bindings for the Wayland protocols a
//! Tizen native window uses on Tizen 10/11.
//!
//! All XMLs in `protocols/` are vendored **verbatim** from upstream
//! Tizen `wayland-extension` (see [`protocols/UPSTREAM.txt`]). We do
//! **not** trim the XMLs — Wayland opcodes are positional, so removing
//! requests in the middle of an interface shifts numbering and the
//! compositor decodes our calls against the wrong request. The full
//! files cost some extra generated code but match the wire format the
//! Tizen compositor expects bit-for-bit.
//!
//! Modules:
//!
//! | Module | XML | What's there |
//! |--------|-----|--------------|
//! | [`xdg_shell_v6`]   | `xdg-shell-unstable-v6.xml` | `zxdg_shell_v6`, `zxdg_surface_v6`, `zxdg_toplevel_v6`, `zxdg_popup_v6`, `zxdg_positioner_v6` |
//! | [`wtz_shell`]      | `wtz-shell.xml`             | `wtz_shell`, `wtz_surface` |
//! | [`wtz_screen`]     | `wtz-screen.xml`            | `wtz_screen`, `wtz_splitscreen`, `wtz_splitscreen_region` |
//! | [`tizen_extension`]| `tizen-extension.xml`       | All Tizen extension interfaces — `tizen_policy`, `tizen_input_device_manager`, `tizen_screenshooter`, `tizen_gesture`, `tizen_visibility`, `tizen_position`, … |
//!
//! Most users want [`tizen-window`](https://crates.io/crates/tizen-window),
//! the safe wrapper. Downstream apps don't depend on this `-sys` crate
//! directly.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::all)]

/// `zxdg_shell_v6` and the surface/toplevel/popup/positioner roles.
pub mod xdg_shell_v6 {
    #[allow(unused_imports)]
    use wayland_client;
    #[allow(unused_imports)]
    use wayland_client::protocol::*;

    #[doc(hidden)]
    pub mod __interfaces {
        #[allow(unused_imports)]
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/xdg-shell-unstable-v6.xml");
    }
    #[allow(unused_imports)]
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/xdg-shell-unstable-v6.xml");
}

/// `wtz_screen` + `wtz_splitscreen` + `wtz_splitscreen_region` +
/// `wtz_shell` + `wtz_surface`. The two upstream files
/// (`wtz-shell.xml`, `wtz-screen.xml`) are merged into one
/// `wtz-shell-bundle.xml` with full content of each so
/// wayland-scanner can resolve `wtz_surface.screen → wtz_screen` in a
/// single pass. Both halves are full-fat upstream verbatim.
pub mod wtz_shell {
    #[allow(unused_imports)]
    use wayland_client;
    #[allow(unused_imports)]
    use wayland_client::protocol::*;

    #[doc(hidden)]
    pub mod __interfaces {
        #[allow(unused_imports)]
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/wtz-shell-bundle.xml");
    }
    #[allow(unused_imports)]
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/wtz-shell-bundle.xml");
}

/// The full Tizen extension surface. Many interfaces; this crate
/// itself only uses `tizen_policy` (for window-visibility requests
/// like `set_type`, `show`, `raise`). The others are emitted so that
/// they're available for other crates that re-export from here, and
/// so that downstream apps can reach for them without re-vendoring.
pub mod tizen_extension {
    #[allow(unused_imports)]
    use wayland_client;
    #[allow(unused_imports)]
    use wayland_client::protocol::*;

    #[doc(hidden)]
    pub mod __interfaces {
        #[allow(unused_imports)]
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/tizen-extension.xml");
    }
    #[allow(unused_imports)]
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/tizen-extension.xml");
}
