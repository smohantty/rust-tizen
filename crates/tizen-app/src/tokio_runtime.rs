use std::future::Future;
use std::sync::Mutex;

use tokio::runtime::{Builder, Runtime};

use crate::app_control::AppControl;
use crate::error::AppError;
use crate::event::LowMemoryStatus;
use crate::lifecycle::{run, Lifecycle};
use crate::service::{run_service, ServiceLifecycle};

/// Async counterpart of [`Lifecycle`](crate::Lifecycle).
pub trait AsyncLifecycle: Send + 'static {
    fn create(&mut self) -> impl Future<Output = Result<(), AppError>> + Send;

    fn terminate(&mut self) -> impl Future<Output = ()> + Send {
        async {}
    }

    fn pause(&mut self) -> impl Future<Output = ()> + Send {
        async {}
    }

    fn resume(&mut self) -> impl Future<Output = ()> + Send {
        async {}
    }

    fn app_control(&mut self, _ctrl: AppControl<'_>) -> impl Future<Output = ()> {
        async {}
    }

    fn low_memory(&mut self, _status: LowMemoryStatus) -> impl Future<Output = ()> + Send {
        async {}
    }

    fn language_changed(&mut self, _language: String) -> impl Future<Output = ()> + Send {
        async {}
    }

    fn region_format_changed(&mut self, _region_format: String) -> impl Future<Output = ()> + Send {
        async {}
    }
}

/// Run an [`AsyncLifecycle`] on a default multi-thread tokio runtime.
pub fn run_async<L: AsyncLifecycle>(lifecycle: L) -> ! {
    run_async_with(lifecycle, |b| b)
}

/// Run an [`AsyncLifecycle`] with a tokio runtime configured by `configure`.
/// `configure` receives a multi-thread [`Builder`] with `enable_all()` set.
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
    fn low_memory(&mut self, status: LowMemoryStatus) {
        let guard = self.inner.get_mut().expect("AsyncLifecycle mutex poisoned");
        self.rt.block_on(guard.low_memory(status));
    }
    fn language_changed(&mut self, language: String) {
        let guard = self.inner.get_mut().expect("AsyncLifecycle mutex poisoned");
        self.rt.block_on(guard.language_changed(language));
    }
    fn region_format_changed(&mut self, region_format: String) {
        let guard = self.inner.get_mut().expect("AsyncLifecycle mutex poisoned");
        self.rt.block_on(guard.region_format_changed(region_format));
    }
}

/// Async counterpart of [`ServiceLifecycle`](crate::ServiceLifecycle).
pub trait AsyncServiceLifecycle: Send + 'static {
    fn create(&mut self) -> impl Future<Output = Result<(), AppError>> + Send;
    fn terminate(&mut self) -> impl Future<Output = ()> + Send {
        async {}
    }
    fn app_control(&mut self, _ctrl: AppControl<'_>) -> impl Future<Output = ()> {
        async {}
    }
    fn low_memory(&mut self, _status: LowMemoryStatus) -> impl Future<Output = ()> + Send {
        async {}
    }
    fn language_changed(&mut self, _language: String) -> impl Future<Output = ()> + Send {
        async {}
    }
    fn region_format_changed(&mut self, _region_format: String) -> impl Future<Output = ()> + Send {
        async {}
    }
}

/// Service-app equivalent of [`run_async`].
pub fn run_service_async<L: AsyncServiceLifecycle>(lifecycle: L) -> ! {
    run_service_async_with(lifecycle, |b| b)
}

/// Service-app equivalent of [`run_async_with`].
pub fn run_service_async_with<L, F>(lifecycle: L, configure: F) -> !
where
    L: AsyncServiceLifecycle,
    F: FnOnce(&mut Builder) -> &mut Builder,
{
    let mut builder = Builder::new_multi_thread();
    builder.enable_all();
    configure(&mut builder);
    let rt = builder
        .build()
        .expect("tizen-app: failed to build tokio runtime");

    let adapter = ServiceAdapter {
        rt,
        inner: Mutex::new(lifecycle),
    };
    run_service(adapter)
}

struct ServiceAdapter<L: AsyncServiceLifecycle> {
    rt: Runtime,
    inner: Mutex<L>,
}

impl<L: AsyncServiceLifecycle> ServiceLifecycle for ServiceAdapter<L> {
    fn create(&mut self) -> Result<(), AppError> {
        let guard = self
            .inner
            .get_mut()
            .expect("AsyncServiceLifecycle mutex poisoned");
        self.rt.block_on(guard.create())
    }
    fn terminate(&mut self) {
        let guard = self
            .inner
            .get_mut()
            .expect("AsyncServiceLifecycle mutex poisoned");
        self.rt.block_on(guard.terminate());
    }
    fn app_control(&mut self, ctrl: AppControl<'_>) {
        let guard = self
            .inner
            .get_mut()
            .expect("AsyncServiceLifecycle mutex poisoned");
        self.rt.block_on(guard.app_control(ctrl));
    }
    fn low_memory(&mut self, status: LowMemoryStatus) {
        let guard = self
            .inner
            .get_mut()
            .expect("AsyncServiceLifecycle mutex poisoned");
        self.rt.block_on(guard.low_memory(status));
    }
    fn language_changed(&mut self, language: String) {
        let guard = self
            .inner
            .get_mut()
            .expect("AsyncServiceLifecycle mutex poisoned");
        self.rt.block_on(guard.language_changed(language));
    }
    fn region_format_changed(&mut self, region_format: String) {
        let guard = self
            .inner
            .get_mut()
            .expect("AsyncServiceLifecycle mutex poisoned");
        self.rt.block_on(guard.region_format_changed(region_format));
    }
}
