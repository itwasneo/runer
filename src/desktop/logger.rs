use chrono::{SecondsFormat, Utc};
use gdk4::glib::Sender;
use log::{set_logger, set_max_level, Level, LevelFilter, Log, Metadata, SetLoggerError};

/// Creates a custom logger. Pins it and then leaks it so that it can be used
/// throughout the application lifetime.
///
/// ---
/// MENTAL NOTE: Should definetely be checked out. Is this a good practice?
pub fn init(tx: Sender<String>) -> Result<(), SetLoggerError> {
    let logger = Box::new(AppLogger { tx: Some(tx) });
    set_logger(Box::leak(logger)).map(|()| set_max_level(LevelFilter::Info))
}

struct AppLogger {
    tx: Option<Sender<String>>,
}

impl Log for AppLogger {
    fn log(&self, record: &log::Record) {
        let timestamp = Utc::now();
        if self.enabled(record.metadata()) {
            if let Some(tx) = &self.tx {
                tx.send(format!(
                    "[{} {} {}] {}",
                    timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
                    record.level(),
                    record.module_path().unwrap_or("unknown"),
                    record.args()
                ))
                .unwrap();
            } else {
                println!(
                    "[{} {} {}] {}",
                    timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
                    record.level(),
                    record.module_path().unwrap_or("unknown"),
                    record.args()
                );
            }
        }
    }
    fn flush(&self) {}
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }
}
