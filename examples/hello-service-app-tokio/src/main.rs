use std::time::Duration;

use log::LevelFilter;
use tizen::app::{AppControl, AppError, AsyncServiceLifecycle};

struct HelloService {
    ticks: u32,
}

impl AsyncServiceLifecycle for HelloService {
    async fn create(&mut self) -> Result<(), AppError> {
        log::info!("hello-service-app-tokio: create");
        // Async work inside a lifecycle callback.
        tokio::time::sleep(Duration::from_millis(50)).await;
        log::info!("hello-service-app-tokio: 50ms async sleep done");
        // Spawn a background task that lives across callbacks on tokio
        // workers, independent of service_app_main's main loop.
        tokio::spawn(async {
            for i in 1..=3 {
                tokio::time::sleep(Duration::from_secs(1)).await;
                log::info!("hello-service-app-tokio: bg heartbeat #{i}");
            }
        });
        Ok(())
    }

    async fn terminate(&mut self) {
        log::info!(
            "hello-service-app-tokio: terminate (ticks={})",
            self.ticks
        );
    }

    async fn app_control(&mut self, ctrl: AppControl<'_>) {
        self.ticks += 1;
        let op = ctrl.operation();
        let uri = ctrl.uri();
        log::info!(
            "hello-service-app-tokio: app_control #{} op={op:?} uri={uri:?}",
            self.ticks
        );
    }
}

struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }
    fn log(&self, record: &log::Record<'_>) {
        eprintln!(
            "[{}] {}: {}",
            record.level(),
            record.target(),
            record.args()
        );
    }
    fn flush(&self) {}
}

static LOGGER: StderrLogger = StderrLogger;

fn main() {
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(LevelFilter::Trace);

    tizen::app::run_service_async_with(HelloService { ticks: 0 }, |b| {
        b.worker_threads(2).thread_name("hello-service-worker")
    });
}
