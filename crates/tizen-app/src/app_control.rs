use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_char;

use tizen_app_sys as sys;

/// Borrowed view of an `app_control_h` passed by the framework into a
/// lifecycle callback.
///
/// The borrow is tied to the callback's invocation — accessor methods can
/// be called freely while the framework still owns the handle. The handle
/// is freed when the callback returns, so do not hold onto a borrowed
/// `AppControl` across awaits unless you've extracted the data you need.
pub struct AppControl<'a> {
    handle: sys::app_control_h,
    _marker: PhantomData<&'a ()>,
}

impl<'a> AppControl<'a> {
    /// SAFETY: `handle` must be non-null and valid for the lifetime `'a`.
    #[cfg_attr(not(tizen), allow(dead_code))]
    pub(crate) unsafe fn from_raw(handle: sys::app_control_h) -> Self {
        Self {
            handle,
            _marker: PhantomData,
        }
    }

    /// The `op` field of the launching intent, if set.
    pub fn operation(&self) -> Option<String> {
        unsafe { read_string(|out| sys::app_control_get_operation(self.handle, out)) }
    }

    /// The `uri` field of the launching intent, if set.
    pub fn uri(&self) -> Option<String> {
        unsafe { read_string(|out| sys::app_control_get_uri(self.handle, out)) }
    }

    /// The app id of the *target* application this control was directed at.
    pub fn app_id(&self) -> Option<String> {
        unsafe { read_string(|out| sys::app_control_get_app_id(self.handle, out)) }
    }

    /// Read a single extra-data string value by key.
    pub fn extra_data(&self, key: &str) -> Option<String> {
        let c_key = CString::new(key).ok()?;
        unsafe {
            read_string(|out| sys::app_control_get_extra_data(self.handle, c_key.as_ptr(), out))
        }
    }
}

/// Common helper: call a `(out: *mut *mut c_char) -> c_int` getter, take
/// ownership of the C-allocated string, and return an owned `String`.
unsafe fn read_string<F>(getter: F) -> Option<String>
where
    F: FnOnce(*mut *mut c_char) -> i32,
{
    let mut ptr: *mut c_char = std::ptr::null_mut();
    let rc = getter(&mut ptr);
    if rc != sys::APP_CONTROL_ERROR_NONE || ptr.is_null() {
        return None;
    }
    let result = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    libc::free(ptr as *mut libc::c_void);
    Some(result)
}

// AppControl<'a> contains a `*mut` handle, which is `!Send + !Sync` by
// default — exactly what we want. The handle isn't thread-safe and is
// freed when the framework callback returns.
