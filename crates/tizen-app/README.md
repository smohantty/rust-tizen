# tizen-app

Safe Rust wrapper over Tizen's app framework: lifecycle callbacks
(`create`/`pause`/`resume`/`terminate`/`app_control`) plus a tokio
integration behind a feature flag.

## Sync usage

```rust
use tizen_app::{AppControl, AppError, Lifecycle};

struct MyApp;

impl Lifecycle for MyApp {
    fn create(&mut self) -> Result<(), AppError> { Ok(()) }
    fn resume(&mut self) { log::info!("resume"); }
    fn app_control(&mut self, ctrl: AppControl<'_>) {
        log::info!("op={:?} uri={:?}", ctrl.operation(), ctrl.uri());
    }
}

fn main() {
    tizen_app::run(MyApp);
}
```

## Async usage (`tokio` feature)

```toml
tizen-app = { version = "0.1", features = ["tokio"] }
```

```rust
use tizen_app::{AppError, AsyncLifecycle};

struct MyApp;

impl AsyncLifecycle for MyApp {
    async fn create(&mut self) -> Result<(), AppError> {
        // any async work here
        Ok(())
    }
    async fn resume(&mut self) { /* … */ }
}

fn main() {
    tizen_app::run_async(MyApp);
}
```

For custom runtime config (worker count, thread names, current-thread
flavour, …) use `run_async_with`:

```rust
tizen_app::run_async_with(MyApp, |b| {
    b.worker_threads(2).thread_name("myapp-worker")
});
```

## Host fallback

On non-Tizen builds, `run` / `run_async` simulate the lifecycle:
`create → resume → (wait for SIGINT/SIGTERM) → pause → terminate`. Useful
for exercising app logic on Linux without flashing a device.

## License

Dual-licensed under Apache-2.0 OR MIT.
