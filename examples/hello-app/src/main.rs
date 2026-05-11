use log::LevelFilter;
use tizen::app::{AppControl, AppError, Lifecycle};
use tizen::dlog::DlogLogger;

struct HelloApp {
    counter: u32,
}

impl Lifecycle for HelloApp {
    fn create(&mut self) -> Result<(), AppError> {
        log::info!("hello-app: create");
        Ok(())
    }

    fn resume(&mut self) {
        self.counter += 1;
        log::info!("hello-app: resume (#{})", self.counter);
    }

    fn pause(&mut self) {
        log::info!("hello-app: pause");
    }

    fn terminate(&mut self) {
        log::info!("hello-app: terminate after {} resumes", self.counter);
    }

    fn app_control(&mut self, ctrl: AppControl<'_>) {
        log::info!(
            "hello-app: app_control op={:?} uri={:?}",
            ctrl.operation(),
            ctrl.uri()
        );
    }
}

fn main() {
    DlogLogger::builder()
        .default_tag("HelloApp")
        .level(LevelFilter::Trace)
        .install()
        .expect("install dlog logger");

    tizen::app::run(HelloApp { counter: 0 });
}
