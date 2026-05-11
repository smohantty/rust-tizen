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

Service apps use `service_app_main` (from `libappcore-agent.so`) — no UI,
no Wayland, no `pause` / `resume`. They run in any headless environment a
service is permitted to start in.

## Async (`tokio` feature)

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

Service-app equivalents — `run_service_async` and `run_service_async_with`
with the [`AsyncServiceLifecycle`] trait — follow the same pattern.

## Host fallback

On non-Tizen builds, the wrappers simulate the lifecycle so consumer apps
can be exercised on plain Linux:

- `run` / `run_async`:   `create → resume → SIGINT/SIGTERM → pause → terminate`
- `run_service` / `run_service_async`:  `create → SIGINT/SIGTERM → terminate`

Useful for iterating on app logic without flashing a device.

## Dependencies

We try to make the dependency story tight. What you pay for:

- **Sync** (`features = ["app"]`): zero async runtime. The tree is
  `log`, `libc`, `tizen-app-sys`, `tizen-app` — four crates total.
- **Async** (`features = ["tokio"]`): adds `tokio` with `default-features
  = false, features = ["rt-multi-thread"]` only, plus `pin-project-lite`.
  No `mio`, `signal-hook-registry`, `tokio-macros`, or proc-macro chain.

The tokio dep is **optional and feature-gated**, so a sync-only consumer
sees no tokio anywhere in their tree.

### Pinning tokio in your app

You don't have to. We re-export it: `tizen_app::tokio` is the same crate
we depend on. For runtime spawning + handles that's enough.

If you need extra tokio features (`time`, `signal`, `macros`, `fs`, …),
add tokio to your own `Cargo.toml`. Cargo's resolver will unify our pin
(`tokio = "1"`, features `rt-multi-thread`) with yours into one
compilation with the union of features — zero duplication.

```toml
[dependencies]
# Async app with custom tokio features:
tizen-app = { version = "0.1", features = ["tokio"] }
tokio = { version = "1", default-features = false, features = ["rt-multi-thread", "time"] }
```

```toml
[dependencies]
# Minimal async app — use re-exported tokio only:
tizen-app = { version = "0.1", features = ["tokio"] }
# (no tokio in your Cargo.toml; `use tizen_app::tokio;` in your code)
```

## License

Dual-licensed under Apache-2.0 OR MIT.
