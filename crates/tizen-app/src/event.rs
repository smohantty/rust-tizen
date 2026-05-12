#[cfg(tizen)]
use std::ffi::CStr;
#[cfg(tizen)]
use std::os::raw::c_char;

#[cfg(tizen)]
use tizen_app_sys as sys;

/// Low-memory pressure reported by Tizen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LowMemoryStatus {
    Normal,
    SoftWarning,
    HardWarning,
    Unknown(i32),
}

#[cfg(tizen)]
impl LowMemoryStatus {
    pub(crate) fn from_raw(raw: sys::app_event_low_memory_status_e) -> Self {
        match raw {
            sys::APP_EVENT_LOW_MEMORY_NORMAL => Self::Normal,
            sys::APP_EVENT_LOW_MEMORY_SOFT_WARNING => Self::SoftWarning,
            sys::APP_EVENT_LOW_MEMORY_HARD_WARNING => Self::HardWarning,
            other => Self::Unknown(other),
        }
    }
}

#[cfg(tizen)]
pub(crate) unsafe fn low_memory_status(
    event_info: sys::app_event_info_h,
) -> Option<LowMemoryStatus> {
    let mut status = 0;
    let rc = sys::app_event_get_low_memory_status(event_info, &mut status);
    if rc != sys::APP_ERROR_NONE {
        return None;
    }
    Some(LowMemoryStatus::from_raw(status))
}

#[cfg(tizen)]
pub(crate) unsafe fn language(event_info: sys::app_event_info_h) -> Option<String> {
    read_event_string(|out| sys::app_event_get_language(event_info, out))
}

#[cfg(tizen)]
pub(crate) unsafe fn region_format(event_info: sys::app_event_info_h) -> Option<String> {
    read_event_string(|out| sys::app_event_get_region_format(event_info, out))
}

#[cfg(tizen)]
unsafe fn read_event_string<F>(getter: F) -> Option<String>
where
    F: FnOnce(*mut *mut c_char) -> i32,
{
    let mut ptr: *mut c_char = std::ptr::null_mut();
    let rc = getter(&mut ptr);
    if rc != sys::APP_ERROR_NONE || ptr.is_null() {
        return None;
    }
    let result = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    libc::free(ptr as *mut libc::c_void);
    Some(result)
}
