use std::time::Duration;

use log::LevelFilter;
use tizen::app::{AppControl, AppError, AsyncLifecycle};
use tizen::dlog::DlogLogger;

struct HelloApp;

impl AsyncLifecycle for HelloApp {
    async fn create(&mut self) -> Result<(), AppError> {
        log::info!("hello-app-tokio: create");
        // Demonstrate async work in a lifecycle callback.
        tokio::time::sleep(Duration::from_millis(50)).await;
        log::info!("hello-app-tokio: 50ms async sleep done");
        Ok(())
    }

    async fn resume(&mut self) {
        log::info!("hello-app-tokio: resume");
        // Spawn a background task that survives across lifecycle transitions
        // by living on tokio's worker threads.
        tokio::spawn(async {
            for i in 1..=3 {
                tokio::time::sleep(Duration::from_secs(1)).await;
                log::info!("hello-app-tokio: bg tick {i}");
            }
        });
    }

    async fn pause(&mut self) {
        log::info!("hello-app-tokio: pause");
    }

    async fn terminate(&mut self) {
        log::info!("hello-app-tokio: terminate");
    }

    async fn app_control(&mut self, ctrl: AppControl<'_>) {
        let op = ctrl.operation();
        let uri = ctrl.uri();
        log::info!("hello-app-tokio: app_control op={op:?} uri={uri:?}");
    }
}

fn main() {
    DlogLogger::builder()
        .default_tag("HelloAppTokio")
        .level(LevelFilter::Trace)
        .install()
        .expect("install dlog logger");

    // Binding owns the runtime; user passes a builder closure to tune it.
    // (Pass `|b| b` for defaults.)
    tizen::app::run_async_with(HelloApp, |b| {
        b.worker_threads(2).thread_name("hello-app-worker")
    });
}
