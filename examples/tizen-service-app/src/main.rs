use log::LevelFilter;
use tizen::app::{AppControl, AppError, ServiceLifecycle};

struct ServiceApp {
    ticks: u32,
}

impl ServiceLifecycle for ServiceApp {
    fn create(&mut self) -> Result<(), AppError> {
        log::info!("tizen-service-app: create");
        Ok(())
    }

    fn terminate(&mut self) {
        log::info!("tizen-service-app: terminate (ticks={})", self.ticks);
    }

    fn app_control(&mut self, ctrl: AppControl<'_>) {
        self.ticks += 1;
        log::info!(
            "tizen-service-app: app_control #{} op={:?} uri={:?}",
            self.ticks,
            ctrl.operation(),
            ctrl.uri()
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

    tizen::app::run_service(ServiceApp { ticks: 0 });
}
