use std::time::Duration;

use log::LevelFilter;
use tizen::app::{AppControl, AppError, AsyncServiceLifecycle};

struct ServiceApp {
    ticks: u32,
}

impl AsyncServiceLifecycle for ServiceApp {
    async fn create(&mut self) -> Result<(), AppError> {
        log::info!("tizen-service-app-tokio: create");
        tokio::time::sleep(Duration::from_millis(50)).await;
        log::info!("tizen-service-app-tokio: 50ms async sleep done");
        tokio::spawn(async {
            for i in 1..=3 {
                tokio::time::sleep(Duration::from_secs(1)).await;
                log::info!("tizen-service-app-tokio: bg heartbeat #{i}");
            }
        });
        Ok(())
    }

    async fn terminate(&mut self) {
        log::info!(
            "tizen-service-app-tokio: terminate (ticks={})",
            self.ticks
        );
    }

    async fn app_control(&mut self, ctrl: AppControl<'_>) {
        self.ticks += 1;
        let op = ctrl.operation();
        let uri = ctrl.uri();
        log::info!(
            "tizen-service-app-tokio: app_control #{} op={op:?} uri={uri:?}",
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

    tizen::app::run_service_async_with(ServiceApp { ticks: 0 }, |b| {
        b.worker_threads(2).thread_name("tizen-service-app-worker")
    });
}
