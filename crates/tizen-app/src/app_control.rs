use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_char;

use tizen_app_sys as sys;

/// Borrowed view of an `app_control_h` passed into a lifecycle callback.
/// Valid only for the duration of the callback; extract any data you
/// need before awaiting.
pub struct AppControl<'a> {
    handle: sys::app_control_h,
    _marker: PhantomData<&'a ()>,
}

impl<'a> AppControl<'a> {
    #[cfg_attr(not(tizen), allow(dead_code))]
    pub(crate) unsafe fn from_raw(handle: sys::app_control_h) -> Self {
        Self {
            handle,
            _marker: PhantomData,
        }
    }

    pub fn operation(&self) -> Option<String> {
        unsafe { read_string(|out| sys::app_control_get_operation(self.handle, out)) }
    }

    pub fn uri(&self) -> Option<String> {
        unsafe { read_string(|out| sys::app_control_get_uri(self.handle, out)) }
    }

    /// The app id of the *target* application this control was directed at.
    pub fn app_id(&self) -> Option<String> {
        unsafe { read_string(|out| sys::app_control_get_app_id(self.handle, out)) }
    }

    pub fn extra_data(&self, key: &str) -> Option<String> {
        let c_key = CString::new(key).ok()?;
        unsafe {
            read_string(|out| sys::app_control_get_extra_data(self.handle, c_key.as_ptr(), out))
        }
    }
}

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
