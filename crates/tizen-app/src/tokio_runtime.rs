use std::future::Future;
use std::sync::Mutex;

use tokio::runtime::{Builder, Runtime};

use crate::app_control::AppControl;
use crate::error::AppError;
use crate::lifecycle::{run, Lifecycle};

/// Async counterpart of [`Lifecycle`](crate::Lifecycle). Methods can `.await`;
/// the runtime is owned by [`run_async`] / [`run_async_with`].
///
/// Note: `app_control` borrows from the framework callback's stack. Extract
/// data from it synchronously before any `.await` if you need to use the
/// values from a spawned task.
pub trait AsyncLifecycle: Send + 'static {
    /// Called once on startup. Return `Err` to abort.
    fn create(&mut self) -> impl Future<Output = Result<(), AppError>> + Send;

    /// Called when the framework is terminating the process.
    fn terminate(&mut self) -> impl Future<Output = ()> + Send {
        async {}
    }

    /// Called when the app is fully obscured.
    fn pause(&mut self) -> impl Future<Output = ()> + Send {
        async {}
    }

    /// Called when the app becomes visible.
    fn resume(&mut self) -> impl Future<Output = ()> + Send {
        async {}
    }

    /// Called for incoming `app_control`. The borrowed handle is valid for
    /// the duration of this call only — extract any data you need before
    /// awaiting on something the framework can't drive.
    ///
    /// Note: the returned future is intentionally **not** `Send`, because
    /// `AppControl<'_>` contains a raw `app_control_h` pointer that isn't
    /// safe to share across threads. Extract data into owned values inside
    /// the future body before any `.await` if you need to use those values
    /// from a `tokio::spawn`.
    fn app_control(&mut self, _ctrl: AppControl<'_>) -> impl Future<Output = ()> {
        async {}
    }
}

/// Run an [`AsyncLifecycle`] with a default multi-thread tokio runtime
/// (`enable_all()`). The runtime is owned by this call and shut down on app
/// exit.
///
/// Equivalent to `run_async_with(lifecycle, |b| b)`.
pub fn run_async<L: AsyncLifecycle>(lifecycle: L) -> ! {
    run_async_with(lifecycle, |b| b)
}

/// Run an [`AsyncLifecycle`] with a tokio runtime configured by `configure`.
///
/// `configure` receives a multi-thread [`Builder`] with `enable_all()`
/// already set. Pass `|b| b` to keep defaults. Use this to set
/// `worker_threads`, `max_blocking_threads`, thread names, etc.
///
/// To get a `current_thread` flavour, replace the builder via
/// `|b| { *b = tokio::runtime::Builder::new_current_thread(); b.enable_all() }`.
pub fn run_async_with<L, F>(lifecycle: L, configure: F) -> !
where
    L: AsyncLifecycle,
    F: FnOnce(&mut Builder) -> &mut Builder,
{
    let mut builder = Builder::new_multi_thread();
    builder.enable_all();
    configure(&mut builder);
    let rt = builder
        .build()
        .expect("tizen-app: failed to build tokio runtime");

    let adapter = Adapter {
        rt,
        inner: Mutex::new(lifecycle),
    };
    run(adapter)
}

/// Bridge: implements sync [`Lifecycle`] by `block_on`ing the user's
/// async methods on the owned runtime.
struct Adapter<L: AsyncLifecycle> {
    rt: Runtime,
    inner: Mutex<L>,
}

impl<L: AsyncLifecycle> Lifecycle for Adapter<L> {
    fn create(&mut self) -> Result<(), AppError> {
        let guard = self.inner.get_mut().expect("AsyncLifecycle mutex poisoned");
        self.rt.block_on(guard.create())
    }
    fn terminate(&mut self) {
        let guard = self.inner.get_mut().expect("AsyncLifecycle mutex poisoned");
        self.rt.block_on(guard.terminate());
    }
    fn pause(&mut self) {
        let guard = self.inner.get_mut().expect("AsyncLifecycle mutex poisoned");
        self.rt.block_on(guard.pause());
    }
    fn resume(&mut self) {
        let guard = self.inner.get_mut().expect("AsyncLifecycle mutex poisoned");
        self.rt.block_on(guard.resume());
    }
    fn app_control(&mut self, ctrl: AppControl<'_>) {
        let guard = self.inner.get_mut().expect("AsyncLifecycle mutex poisoned");
        self.rt.block_on(guard.app_control(ctrl));
    }
}
