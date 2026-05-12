use std::time::Duration;

use log::LevelFilter;
use tizen::app::{AppControl, AppError, AsyncLifecycle, LowMemoryStatus};
use tokio::sync::mpsc;

enum AppEvent {
    LowMemory(LowMemoryStatus),
    LanguageChanged(String),
    RegionFormatChanged(String),
}

struct UiApp {
    event_tx: Option<mpsc::UnboundedSender<AppEvent>>,
}

impl UiApp {
    fn send_event(&self, event: AppEvent) {
        let Some(event_tx) = &self.event_tx else {
            log::warn!("tizen-ui-app-tokio: event channel is not initialized");
            return;
        };

        if event_tx.send(event).is_err() {
            log::warn!("tizen-ui-app-tokio: event task is no longer running");
        }
    }
}

impl AsyncLifecycle for UiApp {
    async fn create(&mut self) -> Result<(), AppError> {
        log::info!("tizen-ui-app-tokio: create");

        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        self.event_tx = Some(event_tx);

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event {
                    AppEvent::LowMemory(status) => {
                        log::info!("tizen-ui-app-tokio: low memory on tokio task: {status:?}");
                    }
                    AppEvent::LanguageChanged(language) => {
                        log::info!(
                            "tizen-ui-app-tokio: language changed on tokio task: {language}"
                        );
                    }
                    AppEvent::RegionFormatChanged(region_format) => {
                        log::info!(
                            "tizen-ui-app-tokio: region format changed on tokio task: {region_format}"
                        );
                    }
                }
            }
        });

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

    async fn low_memory(&mut self, status: LowMemoryStatus) {
        log::info!("tizen-ui-app-tokio: low_memory callback: {status:?}");
        self.send_event(AppEvent::LowMemory(status));
    }

    async fn language_changed(&mut self, language: String) {
        log::info!("tizen-ui-app-tokio: language_changed callback: {language}");
        self.send_event(AppEvent::LanguageChanged(language));
    }

    async fn region_format_changed(&mut self, region_format: String) {
        log::info!("tizen-ui-app-tokio: region_format_changed callback: {region_format}");
        self.send_event(AppEvent::RegionFormatChanged(region_format));
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

    tizen::app::run_async_with(UiApp { event_tx: None }, |b| {
        b.worker_threads(2).thread_name("tizen-ui-app-worker")
    });
}
