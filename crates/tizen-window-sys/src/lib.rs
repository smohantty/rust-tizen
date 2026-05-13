//! Scanner-generated raw client bindings for the Wayland protocols a
//! Tizen native window uses on Tizen 10/11:
//!
//! * `zxdg_shell_v6` — the unstable v6 xdg-shell. Tizen does **not**
//!   advertise the modern stable `xdg_wm_base` on current TV builds,
//!   so v6 is the right target. Confirmed by `tizen-core-wayland`
//!   (`src/tizen-core-wl/tizen_core_wl_surface.c:471`) which calls
//!   `zxdg_shell_v6_get_xdg_surface` + `zxdg_surface_v6_get_toplevel`
//!   on every Tizen window.
//! * `wtz_shell` / `wtz_surface` / `wtz_screen` — Tizen-specific
//!   second role attached to the same `wl_surface`. The two interfaces
//!   live in one merged XML so the scanner resolves the
//!   `wtz_surface.screen` cross-reference in a single pass.
//!
//! Most users want the safe wrapper
//! [`tizen-window`](https://crates.io/crates/tizen-window). Downstream
//! apps don't depend on this `-sys` crate directly.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::all)]

/// `zxdg_shell_v6` + `zxdg_surface_v6` + `zxdg_toplevel_v6` +
/// `zxdg_popup_v6` + `zxdg_positioner_v6`.
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

/// `tizen_policy` — Tizen window-management policy. We expose only
/// the minimal request surface needed to make a window visible
/// (`activate`, `raise`, `set_type`, `show`); the events are declared
/// for parser correctness but the safe wrapper ignores them.
pub mod tizen_policy {
    #[allow(unused_imports)]
    use wayland_client;
    #[allow(unused_imports)]
    use wayland_client::protocol::*;

    #[doc(hidden)]
    pub mod __interfaces {
        #[allow(unused_imports)]
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/tizen-policy.xml");
    }
    #[allow(unused_imports)]
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/tizen-policy.xml");
}

/// `wtz_shell` + `wtz_surface` + `wtz_screen` (trimmed: just the
/// minimal `wtz_screen` slice that `wtz_surface.screen` references).
pub mod wtz_shell {
    #[allow(unused_imports)]
    use wayland_client;
    #[allow(unused_imports)]
    use wayland_client::protocol::*;

    #[doc(hidden)]
    pub mod __interfaces {
        #[allow(unused_imports)]
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/tizen-window.xml");
    }
    #[allow(unused_imports)]
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/tizen-window.xml");
}
