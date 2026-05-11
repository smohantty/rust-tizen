use log::LevelFilter;
use tizen::dlog::{DlogLogger, LogId, TagStrategy};

fn main() {
    DlogLogger::builder()
        .default_tag("HelloDlog")
        .level(LevelFilter::Trace)
        .log_id(LogId::LOG_ID_MAIN)
        .tag_strategy(TagStrategy::TargetThenDefault)
        .install()
        .expect("install dlog logger");

    log::info!("hello from rust-tizen");
    log::warn!("warn: {} retries left", 3);
    log::error!(target: "Network", "boom: {}", 42);
    log::debug!("debug detail: pid={}", std::process::id());
    log::trace!("trace tick");

    println!("hello-dlog: wrote 5 log lines; check `dlogutil hello_dlog:V HelloDlog:V Network:V '*:S'`");
}
