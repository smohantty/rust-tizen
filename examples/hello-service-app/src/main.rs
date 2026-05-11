use log::LevelFilter;
use tizen::app::{AppControl, AppError, ServiceLifecycle};

struct HelloService {
    ticks: u32,
}

impl ServiceLifecycle for HelloService {
    fn create(&mut self) -> Result<(), AppError> {
        log::info!("hello-service-app: create");
        Ok(())
    }

    fn terminate(&mut self) {
        log::info!("hello-service-app: terminate (ticks={})", self.ticks);
    }

    fn app_control(&mut self, ctrl: AppControl<'_>) {
        self.ticks += 1;
        log::info!(
            "hello-service-app: app_control #{} op={:?} uri={:?}",
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

    tizen::app::run_service(HelloService { ticks: 0 });
}
