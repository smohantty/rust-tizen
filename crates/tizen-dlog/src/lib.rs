//! Idiomatic Rust logging into Tizen's dlog system, via the [`log`] facade.
//!
//! ```no_run
//! tizen_dlog::init("MyApp").unwrap();
//! log::info!("hello from rust");
//! log::error!(target: "Network", "boom: {}", 42);
//! ```
//!
//! For more control, use [`DlogLogger::builder`]:
//!
//! ```no_run
//! use tizen_dlog::{DlogLogger, LogId, TagStrategy};
//! use log::LevelFilter;
//!
//! DlogLogger::builder()
//!     .default_tag("MyApp")
//!     .level(LevelFilter::Debug)
//!     .log_id(LogId::LOG_ID_APPS)
//!     .tag_strategy(TagStrategy::TargetThenDefault)
//!     .install()
//!     .unwrap();
//! ```
//!
//! On a Tizen device, view the output with `dlogutil MyApp:* *:S`.

mod logger;
mod priority;
mod tag;

pub use logger::{DlogLogger, DlogLoggerBuilder, LogId};
pub use tag::TagStrategy;

use log::SetLoggerError;

/// Install a `DlogLogger` with sensible defaults and the given tag.
///
/// Equivalent to `DlogLogger::builder().default_tag(tag).install()`.
pub fn init(default_tag: &str) -> Result<(), SetLoggerError> {
    DlogLogger::builder().default_tag(default_tag).install()
}
