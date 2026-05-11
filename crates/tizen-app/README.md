# tizen-app

Safe Rust wrapper over Tizen's app framework: lifecycle callbacks
(`create`/`pause`/`resume`/`terminate`/`app_control`) plus a tokio
integration behind a feature flag.

## UI application (sync)

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

## Service application (sync, headless)

```rust
use tizen_app::{AppControl, AppError, ServiceLifecycle};

struct MyService;

impl ServiceLifecycle for MyService {
    fn create(&mut self) -> Result<(), AppError> { Ok(()) }
    fn app_control(&mut self, ctrl: AppControl<'_>) {
        log::info!("op={:?} uri={:?}", ctrl.operation(), ctrl.uri());
    }
}

fn main() {
    tizen_app::run_service(MyService);
}
```

Service apps use `service_app_main` (from `libappcore-agent.so`) — no
`pause` / `resume`.

## Async (`tokio` feature)

```toml
tizen-app = { git = "https://github.com/smohantty/rust-tizen.git", features = ["tokio"] }
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

Service-app equivalents — `run_service_async` and `run_service_async_with`
with the [`AsyncServiceLifecycle`] trait — follow the same pattern.

## Host fallback

Off-target, the wrappers simulate the lifecycle:

- `run` / `run_async`:   `create → resume → SIGINT/SIGTERM → pause → terminate`
- `run_service` / `run_service_async`:  `create → SIGINT/SIGTERM → terminate`

## Using tokio

`tizen_app::tokio` re-exports our tokio dep, so `use tizen_app::tokio;`
works without pinning tokio in your `Cargo.toml`. If you need extra
tokio features (`time`, `macros`, …), add tokio to your own deps with
those features; Cargo will unify the compile.

## License

Dual-licensed under Apache-2.0 OR MIT.
