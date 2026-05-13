//! Raw client bindings for the Tizen `tizen_input_device_manager` Wayland
//! protocol, generated from a vendored slice of upstream `tizen-extension.xml`
//! (see `protocols/UPSTREAM.txt`) via the `wayland-scanner` proc-macros at
//! compile time.
//!
//! The safe wrapper lives in the `tizen-input` crate; downstream apps
//! should depend on that and only reach for `tizen-input-sys` when they
//! need raw protocol access.
//!
//! ## Why this crate is not `#![no_std]`
//!
//! The project's `-sys` crates are normally `#![no_std]` with no deps. This
//! one is the exception: it is a Wayland *protocol* binding, not a C-library
//! FFI binding, so the FFI plumbing belongs to `wayland-client` /
//! `wayland-backend`. Scanner output uses `String`/`Vec`, hence `std`. The
//! crate still has zero hand-written FFI and zero transitive C deps beyond
//! `libwayland-client.so` (dlopen'd at runtime by `wayland-backend`).

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::all)]

/// Scanner-generated client bindings for `tizen_input_device_manager`
/// (and the `tizen_input_device` interface it returns via `device_add`).
pub mod protocol {
    #[allow(unused_imports)]
    use wayland_client;
    #[allow(unused_imports)]
    use wayland_client::protocol::*;

    /// Low-level `wl_interface` statics. Downstream code rarely touches
    /// these directly; `generate_client_code!` consumes them.
    pub mod __interfaces {
        #[allow(unused_imports)]
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/tizen-input-device-manager.xml");
    }
    #[allow(unused_imports)]
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/tizen-input-device-manager.xml");
}
