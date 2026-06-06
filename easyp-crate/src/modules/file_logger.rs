// file_logger.rs - Persistent file logging system

use std::sync::OnceLock;

struct FileLogger {
    log_path: String,
}

impl FileLogger {
    fn log_path(&self) -> &str {
        &self.log_path
    }
}

static FILE_LOGGER: OnceLock<FileLogger> = OnceLock::new();

pub fn get_log_file_path() -> Option<String> {
    FILE_LOGGER.get().map(|logger| logger.log_path().to_string())
}


