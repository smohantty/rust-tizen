//! Safe wrapper over Tizen's app framework.
//!
//! Implement [`Lifecycle`] on a type and pass it to [`run`]:
//!
//! ```no_run
//! use tizen_app::{AppControl, AppError, Lifecycle};
//!
//! struct MyApp;
//!
//! impl Lifecycle for MyApp {
//!     fn create(&mut self) -> Result<(), AppError> {
//!         log::info!("create");
//!         Ok(())
//!     }
//!     fn resume(&mut self) { log::info!("resume"); }
//!     fn pause(&mut self)  { log::info!("pause"); }
//!     fn terminate(&mut self) { log::info!("terminate"); }
//!     fn app_control(&mut self, ctrl: AppControl<'_>) {
//!         log::info!("app_control: op={:?} uri={:?}", ctrl.operation(), ctrl.uri());
//!     }
//! }
//!
//! fn main() {
//!     tizen_app::run(MyApp);
//! }
//! ```
//!
//! With the `tokio` feature, see [`AsyncLifecycle`] and [`run_async`] /
//! [`run_async_with`] for async lifecycle methods.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod app_control;
mod error;
mod lifecycle;
mod service;

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
mod tokio_runtime;

pub use app_control::AppControl;
pub use error::AppError;
pub use lifecycle::{run, Lifecycle};
pub use service::{run_service, ServiceLifecycle};

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub use tokio_runtime::{
    run_async, run_async_with, run_service_async, run_service_async_with, AsyncLifecycle,
    AsyncServiceLifecycle,
};
