use log::{Level, Log, Metadata, Record};

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if record.level() <= Level::Warn {
                eprintln!("[{}] {}", record.level(), record.args());
            } else {
                println!("[{}] {}", record.level(), record.args());
            }
        }
    }

    fn flush(&self) {}
}
