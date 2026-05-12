use crate::app_control::AppControl;
use crate::error::AppError;
use crate::event::LowMemoryStatus;

/// Synchronous service-app lifecycle callbacks (headless, no pause/resume).
pub trait ServiceLifecycle {
    fn create(&mut self) -> Result<(), AppError>;
    fn terminate(&mut self) {}
    fn app_control(&mut self, _ctrl: AppControl<'_>) {}
    fn low_memory(&mut self, _status: LowMemoryStatus) {}
    fn language_changed(&mut self, _language: String) {}
    fn region_format_changed(&mut self, _region_format: String) {}
}

/// Run a service-app loop via `service_app_main`. Never returns.
pub fn run_service<L: ServiceLifecycle + 'static>(lifecycle: L) -> ! {
    #[cfg(tizen)]
    {
        tizen::run_service(lifecycle)
    }
    #[cfg(not(tizen))]
    {
        host_fallback::run_service(lifecycle)
    }
}

#[cfg(tizen)]
mod tizen {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int, c_void};
    use std::panic;

    use tizen_app_sys as sys;

    use crate::event;

    use super::{AppControl, ServiceLifecycle};

    struct CallbackState<L> {
        lifecycle: L,
        event_handlers: [sys::app_event_handler_h; 3],
    }

    impl<L> CallbackState<L> {
        fn new(lifecycle: L) -> Self {
            Self {
                lifecycle,
                event_handlers: [std::ptr::null_mut(); 3],
            }
        }
    }

    pub(super) fn run_service<L: ServiceLifecycle + 'static>(lifecycle: L) -> ! {
        let mut state = Box::new(CallbackState::new(lifecycle));
        add_event_handlers::<L>(&mut state);
        let state = Box::into_raw(state);

        let mut callbacks = sys::service_app_lifecycle_callback_s {
            create: Some(trampoline_create::<L>),
            terminate: Some(trampoline_terminate::<L>),
            app_control: Some(trampoline_app_control::<L>),
        };

        let args: Vec<CString> = std::env::args()
            .map(|s| CString::new(s).unwrap_or_default())
            .collect();
        let mut argv: Vec<*mut c_char> = args.iter().map(|s| s.as_ptr() as *mut _).collect();
        argv.push(std::ptr::null_mut());

        let rc = unsafe {
            sys::service_app_main(
                args.len() as c_int,
                argv.as_mut_ptr(),
                &mut callbacks,
                state as *mut c_void,
            )
        };

        let mut state = unsafe { Box::from_raw(state) };
        remove_event_handlers(&mut state.event_handlers);

        drop(state);
        std::process::exit(rc);
    }

    unsafe extern "C" fn trampoline_create<L: ServiceLifecycle>(
        user_data: *mut c_void,
    ) -> sys::bool_t {
        let state = &mut *(user_data as *mut CallbackState<L>);
        match panic::catch_unwind(panic::AssertUnwindSafe(|| state.lifecycle.create())) {
            Ok(Ok(())) => 1,
            Ok(Err(e)) => {
                log::error!("tizen-app: service create failed: {e}");
                0
            }
            Err(_) => {
                log::error!("tizen-app: service create panicked");
                0
            }
        }
    }

    unsafe extern "C" fn trampoline_terminate<L: ServiceLifecycle>(user_data: *mut c_void) {
        let state = &mut *(user_data as *mut CallbackState<L>);
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| state.lifecycle.terminate()));
    }
    unsafe extern "C" fn trampoline_app_control<L: ServiceLifecycle>(
        ac: sys::app_control_h,
        user_data: *mut c_void,
    ) {
        let state = &mut *(user_data as *mut CallbackState<L>);
        let ctrl = AppControl::from_raw(ac);
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            state.lifecycle.app_control(ctrl)
        }));
    }

    fn add_event_handlers<L: ServiceLifecycle>(state: &mut CallbackState<L>) {
        let state_ptr = state as *mut CallbackState<L>;
        add_event_handler(
            &mut state.event_handlers[0],
            sys::APP_EVENT_LOW_MEMORY,
            Some(trampoline_low_memory::<L>),
            state_ptr,
        );
        add_event_handler(
            &mut state.event_handlers[1],
            sys::APP_EVENT_LANGUAGE_CHANGED,
            Some(trampoline_language_changed::<L>),
            state_ptr,
        );
        add_event_handler(
            &mut state.event_handlers[2],
            sys::APP_EVENT_REGION_FORMAT_CHANGED,
            Some(trampoline_region_format_changed::<L>),
            state_ptr,
        );
    }

    fn add_event_handler<L: ServiceLifecycle>(
        handler: &mut sys::app_event_handler_h,
        event_type: sys::app_event_type_e,
        callback: sys::app_event_cb,
        state: *mut CallbackState<L>,
    ) {
        let rc = unsafe {
            sys::service_app_add_event_handler(handler, event_type, callback, state as *mut c_void)
        };
        if rc != sys::APP_ERROR_NONE {
            log::warn!("tizen-app: failed to add service app event handler {event_type}: {rc}");
        }
    }

    fn remove_event_handlers(handlers: &mut [sys::app_event_handler_h; 3]) {
        for handler in handlers {
            if !handler.is_null() {
                let rc = unsafe { sys::service_app_remove_event_handler(*handler) };
                if rc != sys::APP_ERROR_NONE {
                    log::warn!("tizen-app: failed to remove service app event handler: {rc}");
                }
                *handler = std::ptr::null_mut();
            }
        }
    }

    unsafe extern "C" fn trampoline_low_memory<L: ServiceLifecycle>(
        event_info: sys::app_event_info_h,
        user_data: *mut c_void,
    ) {
        let Some(status) = event::low_memory_status(event_info) else {
            log::warn!("tizen-app: failed to read service low-memory event status");
            return;
        };
        let state = &mut *(user_data as *mut CallbackState<L>);
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            state.lifecycle.low_memory(status)
        }));
    }

    unsafe extern "C" fn trampoline_language_changed<L: ServiceLifecycle>(
        event_info: sys::app_event_info_h,
        user_data: *mut c_void,
    ) {
        let Some(language) = event::language(event_info) else {
            log::warn!("tizen-app: failed to read service language-changed event");
            return;
        };
        let state = &mut *(user_data as *mut CallbackState<L>);
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            state.lifecycle.language_changed(language)
        }));
    }

    unsafe extern "C" fn trampoline_region_format_changed<L: ServiceLifecycle>(
        event_info: sys::app_event_info_h,
        user_data: *mut c_void,
    ) {
        let Some(region_format) = event::region_format(event_info) else {
            log::warn!("tizen-app: failed to read service region-format-changed event");
            return;
        };
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let state = &mut *(user_data as *mut CallbackState<L>);
            state.lifecycle.region_format_changed(region_format)
        }));
    }
}

#[cfg(not(tizen))]
mod host_fallback {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::ServiceLifecycle;

    static SHUTDOWN: AtomicBool = AtomicBool::new(false);

    extern "C" fn handle_signal(_: i32) {
        SHUTDOWN.store(true, Ordering::SeqCst);
    }

    pub(super) fn run_service<L: ServiceLifecycle + 'static>(mut lifecycle: L) -> ! {
        log::info!("tizen-app: service host fallback — simulating lifecycle");

        let handler = handle_signal as *const () as libc::sighandler_t;
        unsafe {
            libc::signal(libc::SIGINT, handler);
            libc::signal(libc::SIGTERM, handler);
        }

        if let Err(e) = lifecycle.create() {
            log::error!("tizen-app: service create failed: {e}");
            std::process::exit(1);
        }

        while !SHUTDOWN.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        lifecycle.terminate();
        std::process::exit(0);
    }
}
