use log::Level;
use tizen_dlog_sys::log_priority;

pub(crate) fn level_to_priority(level: Level) -> log_priority {
    match level {
        Level::Error => log_priority::DLOG_ERROR,
        Level::Warn => log_priority::DLOG_WARN,
        Level::Info => log_priority::DLOG_INFO,
        Level::Debug => log_priority::DLOG_DEBUG,
        Level::Trace => log_priority::DLOG_VERBOSE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_level() {
        assert_eq!(level_to_priority(Level::Error), log_priority::DLOG_ERROR);
        assert_eq!(level_to_priority(Level::Warn), log_priority::DLOG_WARN);
        assert_eq!(level_to_priority(Level::Info), log_priority::DLOG_INFO);
        assert_eq!(level_to_priority(Level::Debug), log_priority::DLOG_DEBUG);
        assert_eq!(level_to_priority(Level::Trace), log_priority::DLOG_VERBOSE);
    }
}
