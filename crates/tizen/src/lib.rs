//! Idiomatic Rust bindings to Tizen platform APIs.
//!
//! This is an *umbrella crate*: each Tizen subsystem (logging, app lifecycle,
//! sensors, …) lives in its own focused crate (`tizen-dlog`, `tizen-app`, …)
//! and this crate re-exports them behind cargo features so you can opt in to
//! exactly what you need.
//!
//! ## Quick start
//!
//! ```toml
//! [dependencies]
//! tizen = { version = "0.1", features = ["dlog"] }
//! log = "0.4"
//! ```
//!
//! ```ignore
//! tizen::dlog::init("MyApp").unwrap();
//! log::info!("hello from rust");
//! ```
//!
//! ## Available bindings
//!
//! | Feature        | Module          | Status     | Backing crate                |
//! |----------------|-----------------|------------|------------------------------|
//! | `dlog`         | [`dlog`]        | 🟢 alpha   | [`tizen-dlog`]               |
//! | `app`          | `app`           | 📝 planned | `tizen-app`                  |
//! | `system-info`  | `system_info`   | 📝 planned | `tizen-system-info`          |
//! | `sensor`       | `sensor`        | 📝 planned | `tizen-sensor`               |
//! | `bundle`       | `bundle`        | 📝 planned | `tizen-bundle`               |
//! | `notification` | `notification`  | 📝 planned | `tizen-notification`         |
//!
//! [`tizen-dlog`]: https://crates.io/crates/tizen-dlog
//!
//! ## Direct dependency alternative
//!
//! If you want only one binding and prefer skipping the umbrella, depend on
//! the focused crate directly:
//!
//! ```toml
//! [dependencies]
//! tizen-dlog = "0.1"
//! ```
//!
//! Both paths work; the umbrella exists for ergonomic discovery and a single
//! version pin across multiple bindings.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

/// Logging into Tizen's dlog system via the [`log`](https://crates.io/crates/log) facade.
///
/// Re-exported from the [`tizen-dlog`](https://crates.io/crates/tizen-dlog) crate.
/// Enable with the `dlog` feature.
#[cfg(feature = "dlog")]
#[cfg_attr(docsrs, doc(cfg(feature = "dlog")))]
pub use tizen_dlog as dlog;
