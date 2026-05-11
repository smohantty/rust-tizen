use log::LevelFilter;
use tizen::app::{AppControl, AppError, Lifecycle};

struct UiApp {
    counter: u32,
}

impl Lifecycle for UiApp {
    fn create(&mut self) -> Result<(), AppError> {
        log::info!("tizen-ui-app: create");
        Ok(())
    }

    fn resume(&mut self) {
        self.counter += 1;
        log::info!("tizen-ui-app: resume (#{})", self.counter);
    }

    fn pause(&mut self) {
        log::info!("tizen-ui-app: pause");
    }

    fn terminate(&mut self) {
        log::info!("tizen-ui-app: terminate after {} resumes", self.counter);
    }

    fn app_control(&mut self, ctrl: AppControl<'_>) {
        log::info!(
            "tizen-ui-app: app_control op={:?} uri={:?}",
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

    tizen::app::run(UiApp { counter: 0 });
}
