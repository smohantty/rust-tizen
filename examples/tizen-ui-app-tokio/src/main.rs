use std::time::Duration;

use log::LevelFilter;
use tizen::app::{AppControl, AppError, AsyncLifecycle};

struct UiApp;

impl AsyncLifecycle for UiApp {
    async fn create(&mut self) -> Result<(), AppError> {
        log::info!("tizen-ui-app-tokio: create");
        tokio::time::sleep(Duration::from_millis(50)).await;
        log::info!("tizen-ui-app-tokio: 50ms async sleep done");
        Ok(())
    }

    async fn resume(&mut self) {
        log::info!("tizen-ui-app-tokio: resume");
        tokio::spawn(async {
            for i in 1..=3 {
                tokio::time::sleep(Duration::from_secs(1)).await;
                log::info!("tizen-ui-app-tokio: bg tick {i}");
            }
        });
    }

    async fn pause(&mut self) {
        log::info!("tizen-ui-app-tokio: pause");
    }

    async fn terminate(&mut self) {
        log::info!("tizen-ui-app-tokio: terminate");
    }

    async fn app_control(&mut self, ctrl: AppControl<'_>) {
        let op = ctrl.operation();
        let uri = ctrl.uri();
        log::info!("tizen-ui-app-tokio: app_control op={op:?} uri={uri:?}");
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

    tizen::app::run_async_with(UiApp, |b| {
        b.worker_threads(2).thread_name("tizen-ui-app-worker")
    });
}
