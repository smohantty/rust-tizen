//! Scanner-generated client bindings for `tizen_screenshooter`.
//!
//! Generated at compile time from `protocols/tizen-screenshooter.xml` via
//! the `wayland-scanner` proc-macros. `wl_output` / `wl_buffer` references
//! are resolved against `wayland-client`'s core protocol module.

#[allow(unused_imports)]
use wayland_client;
#[allow(unused_imports)]
use wayland_client::protocol::*;

#[doc(hidden)]
pub mod __interfaces {
    #[allow(unused_imports)]
    use wayland_client::protocol::__interfaces::*;
    wayland_scanner::generate_interfaces!("protocols/tizen-screenshooter.xml");
}
#[allow(unused_imports)]
use self::__interfaces::*;

wayland_scanner::generate_client_code!("protocols/tizen-screenshooter.xml");
