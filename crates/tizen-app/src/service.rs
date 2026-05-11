use crate::app_control::AppControl;
use crate::error::AppError;

/// Synchronous service-app lifecycle callbacks (headless, no pause/resume).
pub trait ServiceLifecycle {
    fn create(&mut self) -> Result<(), AppError>;
    fn terminate(&mut self) {}
    fn app_control(&mut self, _ctrl: AppControl<'_>) {}
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

    use super::{AppControl, ServiceLifecycle};

    pub(super) fn run_service<L: ServiceLifecycle + 'static>(lifecycle: L) -> ! {
        let lifecycle = Box::into_raw(Box::new(lifecycle));

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
                lifecycle as *mut c_void,
            )
        };

        unsafe {
            drop(Box::from_raw(lifecycle));
        }
        std::process::exit(rc);
    }

    unsafe extern "C" fn trampoline_create<L: ServiceLifecycle>(
        user_data: *mut c_void,
    ) -> sys::bool_t {
        let l = &mut *(user_data as *mut L);
        match panic::catch_unwind(panic::AssertUnwindSafe(|| l.create())) {
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
        let l = &mut *(user_data as *mut L);
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| l.terminate()));
    }
    unsafe extern "C" fn trampoline_app_control<L: ServiceLifecycle>(
        ac: sys::app_control_h,
        user_data: *mut c_void,
    ) {
        let l = &mut *(user_data as *mut L);
        let ctrl = AppControl::from_raw(ac);
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| l.app_control(ctrl)));
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
