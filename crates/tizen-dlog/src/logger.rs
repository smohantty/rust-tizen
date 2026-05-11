use std::ffi::CString;

use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};
use tizen_dlog_sys as sys;

use crate::priority::level_to_priority;
use crate::tag::{self, TagStrategy};

/// Re-exported `log_id_t` so callers don't need a direct dep on `tizen-dlog-sys`.
pub use sys::log_id_t as LogId;

/// A logger that forwards `log` records to Tizen's dlog system.
pub struct DlogLogger {
    default_tag: CString,
    level: LevelFilter,
    log_id: LogId,
    tag_strategy: TagStrategy,
}

impl DlogLogger {
    /// Start a builder. Call [`DlogLoggerBuilder::install`] to register as the global logger.
    pub fn builder() -> DlogLoggerBuilder {
        DlogLoggerBuilder::default()
    }
}

/// Builder for [`DlogLogger`].
pub struct DlogLoggerBuilder {
    default_tag: String,
    level: LevelFilter,
    log_id: LogId,
    tag_strategy: TagStrategy,
}

impl Default for DlogLoggerBuilder {
    fn default() -> Self {
        Self {
            default_tag: String::from("RUST"),
            level: LevelFilter::Info,
            log_id: LogId::LOG_ID_MAIN,
            tag_strategy: TagStrategy::default(),
        }
    }
}

impl DlogLoggerBuilder {
    /// Tag used when the record carries no explicit `target` override (or when
    /// [`TagStrategy::AlwaysDefault`] is selected). Default: `"RUST"`.
    pub fn default_tag(mut self, tag: impl Into<String>) -> Self {
        self.default_tag = tag.into();
        self
    }

    /// Maximum level to forward. Records above this filter are dropped before any FFI call.
    /// Default: [`LevelFilter::Info`].
    pub fn level(mut self, level: LevelFilter) -> Self {
        self.level = level;
        self
    }

    /// Which dlog buffer to write to. Default: [`LogId::LOG_ID_MAIN`].
    /// App developers commonly want [`LogId::LOG_ID_APPS`].
    pub fn log_id(mut self, id: LogId) -> Self {
        self.log_id = id;
        self
    }

    /// How to derive the dlog tag from each record. Default: [`TagStrategy::TargetThenDefault`].
    pub fn tag_strategy(mut self, s: TagStrategy) -> Self {
        self.tag_strategy = s;
        self
    }

    /// Build the [`DlogLogger`] without installing it. Useful for tests.
    pub fn build(self) -> DlogLogger {
        DlogLogger {
            default_tag: sanitise_to_cstring(&self.default_tag),
            level: self.level,
            log_id: self.log_id,
            tag_strategy: self.tag_strategy,
        }
    }

    /// Build and install as the global logger via [`log::set_boxed_logger`].
    pub fn install(self) -> Result<(), SetLoggerError> {
        let logger = self.build();
        let max_level = logger.level;
        unsafe {
            sys::dlog_set_minimum_priority(crate::priority::level_filter_to_priority(max_level));
        }
        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(max_level);
        Ok(())
    }
}

impl Log for DlogLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let default_tag_str = self
            .default_tag
            .to_str()
            .expect("default_tag is sanitised UTF-8 with no NULs");
        let tag_str = tag::resolve(self.tag_strategy, record.target(), default_tag_str);

        // Reuse the pre-built default-tag CString when possible to avoid allocating
        // on the hot path. Otherwise build a fresh CString that lives until the end
        // of this function, keeping `tag_ptr` valid for the FFI call.
        let tag_owned;
        let tag_ptr = if tag_str == default_tag_str {
            self.default_tag.as_ptr()
        } else {
            tag_owned = sanitise_to_cstring(tag_str);
            tag_owned.as_ptr()
        };

        let msg = sanitise_to_cstring(&record.args().to_string());
        let prio = level_to_priority(record.level());

        unsafe {
            sys::__dlog_print(self.log_id, prio, tag_ptr, c"%s".as_ptr(), msg.as_ptr());
        }
    }

    fn flush(&self) {}
}

/// Build a `CString` from a `&str`, replacing any interior NUL bytes with spaces so
/// that logging never panics on user data.
fn sanitise_to_cstring(s: &str) -> CString {
    if s.as_bytes().contains(&0) {
        let cleaned: String = s.chars().map(|c| if c == '\0' { ' ' } else { c }).collect();
        CString::new(cleaned).expect("NULs were replaced")
    } else {
        CString::new(s).expect("no NULs present")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitise_replaces_interior_nul() {
        let c = sanitise_to_cstring("hello\0world");
        assert_eq!(c.to_bytes(), b"hello world");
    }

    #[test]
    fn sanitise_passes_through_clean_str() {
        let c = sanitise_to_cstring("hello world");
        assert_eq!(c.to_bytes(), b"hello world");
    }

    #[test]
    fn sanitise_handles_empty() {
        let c = sanitise_to_cstring("");
        assert_eq!(c.to_bytes(), b"");
    }

    #[test]
    fn builder_defaults() {
        let logger = DlogLogger::builder().build();
        assert_eq!(logger.level, LevelFilter::Info);
        assert_eq!(logger.log_id, LogId::LOG_ID_MAIN);
        assert_eq!(logger.tag_strategy, TagStrategy::TargetThenDefault);
        assert_eq!(logger.default_tag.to_bytes(), b"RUST");
    }

    #[test]
    fn builder_customisation() {
        let logger = DlogLogger::builder()
            .default_tag("MyApp")
            .level(LevelFilter::Debug)
            .log_id(LogId::LOG_ID_APPS)
            .tag_strategy(TagStrategy::AlwaysDefault)
            .build();
        assert_eq!(logger.level, LevelFilter::Debug);
        assert_eq!(logger.log_id, LogId::LOG_ID_APPS);
        assert_eq!(logger.tag_strategy, TagStrategy::AlwaysDefault);
        assert_eq!(logger.default_tag.to_bytes(), b"MyApp");
    }
}
