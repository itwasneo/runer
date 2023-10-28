use chrono::{SecondsFormat, Utc};
use gdk4::glib::Sender;
use log::{set_logger, set_max_level, Level, LevelFilter, Log, Metadata, SetLoggerError};

pub fn init(tx: Sender<String>) -> Result<(), SetLoggerError> {
    let logger = Box::new(MyLogger { tx: Some(tx) });
    set_logger(Box::leak(logger)).map(|()| set_max_level(LevelFilter::Info))
}

struct MyLogger {
    tx: Option<Sender<String>>,
}

impl Log for MyLogger {
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
