//! Umbrella crate re-exporting Tizen subsystem bindings behind cargo features.
//!
//! ```toml
//! [dependencies]
//! tizen = { git = "https://github.com/smohantty/rust-tizen.git", features = ["dlog"] }
//! ```
//!
//! | Feature     | Module        | Backing crate            |
//! |-------------|---------------|--------------------------|
//! | `dlog`      | [`dlog`]      | [`tizen-dlog`]           |
//! | `app`       | [`app`]       | [`tizen-app`]            |
//! | `app-tokio` | [`app`]       | [`tizen-app`] + tokio    |
//! | `input`     | [`input`]     | [`tizen-input`]          |
//! | `screenshot`| [`screenshot`]| [`tizen-screenshot`]     |
//!
//! [`tizen-dlog`]: https://crates.io/crates/tizen-dlog
//! [`tizen-app`]: https://crates.io/crates/tizen-app
//! [`tizen-input`]: https://crates.io/crates/tizen-input
//! [`tizen-screenshot`]: https://crates.io/crates/tizen-screenshot

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

/// Logging into Tizen's dlog system via the [`log`](https://crates.io/crates/log) facade.
#[cfg(feature = "dlog")]
#[cfg_attr(docsrs, doc(cfg(feature = "dlog")))]
pub use tizen_dlog as dlog;

/// App lifecycle (`ui_app_main`) + `app_control` intents. Optional tokio
/// runtime under the `app-tokio` feature.
#[cfg(feature = "app")]
#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
pub use tizen_app as app;

/// Inject key/touch/pointer events into the Tizen compositor via the
/// `tizen_input_device_manager` Wayland protocol.
#[cfg(feature = "input")]
#[cfg_attr(docsrs, doc(cfg(feature = "input")))]
pub use tizen_input as input;

/// One-shot screenshot capture from the Tizen compositor via the
/// `tizen_screenshooter` Wayland protocol + TBM-backed buffers.
#[cfg(feature = "screenshot")]
#[cfg_attr(docsrs, doc(cfg(feature = "screenshot")))]
pub use tizen_screenshot as screenshot;
