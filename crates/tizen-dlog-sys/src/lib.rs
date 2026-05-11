//! Raw FFI bindings to Tizen's `libdlog`.
//!
//! These declarations mirror `<dlog.h>` from the Tizen platform SDK.
//! No safety, no formatting — see the `tizen-dlog` crate for the safe wrapper.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![no_std]

use core::ffi::{c_char, c_int};

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum log_priority {
    DLOG_UNKNOWN = 0,
    DLOG_DEFAULT = 1,
    DLOG_VERBOSE = 2,
    DLOG_DEBUG = 3,
    DLOG_INFO = 4,
    DLOG_WARN = 5,
    DLOG_ERROR = 6,
    DLOG_FATAL = 7,
    DLOG_SILENT = 8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum log_id_t {
    LOG_ID_INVALID = -1,
    LOG_ID_MAIN = 0,
    LOG_ID_RADIO = 1,
    LOG_ID_SYSTEM = 2,
    LOG_ID_APPS = 3,
    LOG_ID_KMSG = 4,
    LOG_ID_SYSLOG = 5,
}

// Link `libdlog.so` only when building for Tizen (cargo-tizen sets `cfg(tizen)`).
//
// Only symbols that exist on every supported Tizen version are declared
// here. Newer-version-only symbols (e.g. `dlog_set_minimum_priority`,
// introduced in Tizen 11) are looked up via `dlsym` from the safe wrapper
// so a single binary loads cleanly on both Tizen 10 and 11.
#[cfg_attr(tizen, link(name = "dlog", kind = "dylib"))]
extern "C" {
    pub fn dlog_print(prio: log_priority, tag: *const c_char, fmt: *const c_char, ...) -> c_int;

    pub fn __dlog_print(
        log_id: log_id_t,
        prio: log_priority,
        tag: *const c_char,
        fmt: *const c_char,
        ...
    ) -> c_int;
}
