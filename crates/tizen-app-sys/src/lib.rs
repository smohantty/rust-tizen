#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![no_std]

use core::ffi::{c_char, c_int, c_void};

pub type bool_t = u8;

#[repr(C)]
pub struct app_control_s {
    _private: [u8; 0],
}
pub type app_control_h = *mut app_control_s;

#[repr(C)]
pub struct app_event_handler_s {
    _private: [u8; 0],
}
pub type app_event_handler_h = *mut app_event_handler_s;

#[repr(C)]
pub struct app_event_info_s {
    _private: [u8; 0],
}
pub type app_event_info_h = *mut app_event_info_s;

pub type app_event_type_e = c_int;
pub const APP_EVENT_LOW_MEMORY: app_event_type_e = 0;
pub const APP_EVENT_LANGUAGE_CHANGED: app_event_type_e = 2;
pub const APP_EVENT_REGION_FORMAT_CHANGED: app_event_type_e = 4;

pub type app_event_low_memory_status_e = c_int;
pub const APP_EVENT_LOW_MEMORY_NORMAL: app_event_low_memory_status_e = 0x01;
pub const APP_EVENT_LOW_MEMORY_SOFT_WARNING: app_event_low_memory_status_e = 0x02;
pub const APP_EVENT_LOW_MEMORY_HARD_WARNING: app_event_low_memory_status_e = 0x04;

pub type app_create_cb = Option<unsafe extern "C" fn(user_data: *mut c_void) -> bool_t>;
pub type app_terminate_cb = Option<unsafe extern "C" fn(user_data: *mut c_void)>;
pub type app_pause_cb = Option<unsafe extern "C" fn(user_data: *mut c_void)>;
pub type app_resume_cb = Option<unsafe extern "C" fn(user_data: *mut c_void)>;
pub type app_control_cb =
    Option<unsafe extern "C" fn(app_control: app_control_h, user_data: *mut c_void)>;
pub type app_event_cb =
    Option<unsafe extern "C" fn(event_info: app_event_info_h, user_data: *mut c_void)>;

#[repr(C)]
pub struct ui_app_lifecycle_callback_s {
    pub create: app_create_cb,
    pub terminate: app_terminate_cb,
    pub pause: app_pause_cb,
    pub resume: app_resume_cb,
    pub app_control: app_control_cb,
}

pub const APP_ERROR_NONE: c_int = 0;
pub const APP_CONTROL_ERROR_NONE: c_int = 0;

#[cfg_attr(tizen, link(name = "capi-appfw-application", kind = "dylib"))]
extern "C" {
    pub fn ui_app_main(
        argc: c_int,
        argv: *mut *mut c_char,
        callback: *mut ui_app_lifecycle_callback_s,
        user_data: *mut c_void,
    ) -> c_int;

    pub fn ui_app_exit();

    pub fn ui_app_add_event_handler(
        event_handler: *mut app_event_handler_h,
        event_type: app_event_type_e,
        callback: app_event_cb,
        user_data: *mut c_void,
    ) -> c_int;

    pub fn ui_app_remove_event_handler(event_handler: app_event_handler_h) -> c_int;
}

#[cfg_attr(tizen, link(name = "capi-appfw-app-control", kind = "dylib"))]
extern "C" {
    pub fn app_control_clone(clone: *mut app_control_h, app_control: app_control_h) -> c_int;
    pub fn app_control_destroy(app_control: app_control_h) -> c_int;
    pub fn app_control_get_operation(
        app_control: app_control_h,
        operation: *mut *mut c_char,
    ) -> c_int;
    pub fn app_control_get_uri(app_control: app_control_h, uri: *mut *mut c_char) -> c_int;
    pub fn app_control_get_app_id(app_control: app_control_h, app_id: *mut *mut c_char) -> c_int;
    pub fn app_control_get_extra_data(
        app_control: app_control_h,
        key: *const c_char,
        value: *mut *mut c_char,
    ) -> c_int;
}

#[cfg_attr(tizen, link(name = "capi-appfw-app-common", kind = "dylib"))]
extern "C" {
    pub fn app_event_get_low_memory_status(
        event_info: app_event_info_h,
        status: *mut app_event_low_memory_status_e,
    ) -> c_int;
    pub fn app_event_get_language(
        event_info: app_event_info_h,
        language: *mut *mut c_char,
    ) -> c_int;
    pub fn app_event_get_region_format(
        event_info: app_event_info_h,
        region: *mut *mut c_char,
    ) -> c_int;
}

// Service app lifecycle (headless, no UI). Service apps live in
// `libappcore-agent.so` rather than `libcapi-appfw-application.so`.
pub type service_app_create_cb = Option<unsafe extern "C" fn(user_data: *mut c_void) -> bool_t>;
pub type service_app_terminate_cb = Option<unsafe extern "C" fn(user_data: *mut c_void)>;
pub type service_app_control_cb =
    Option<unsafe extern "C" fn(app_control: app_control_h, user_data: *mut c_void)>;

#[repr(C)]
pub struct service_app_lifecycle_callback_s {
    pub create: service_app_create_cb,
    pub terminate: service_app_terminate_cb,
    pub app_control: service_app_control_cb,
}

#[cfg_attr(tizen, link(name = "appcore-agent", kind = "dylib"))]
extern "C" {
    pub fn service_app_main(
        argc: c_int,
        argv: *mut *mut c_char,
        callback: *mut service_app_lifecycle_callback_s,
        user_data: *mut c_void,
    ) -> c_int;

    pub fn service_app_exit();

    pub fn service_app_add_event_handler(
        event_handler: *mut app_event_handler_h,
        event_type: app_event_type_e,
        callback: app_event_cb,
        user_data: *mut c_void,
    ) -> c_int;

    pub fn service_app_remove_event_handler(event_handler: app_event_handler_h) -> c_int;
}
