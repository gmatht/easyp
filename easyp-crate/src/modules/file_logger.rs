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

pub fn init_file_logger(log_path: &str) -> Result<(), String> {
    FILE_LOGGER.set(FileLogger { log_path: log_path.to_string() })
        .map_err(|_| "File logger already initialized".to_string())
}

pub fn write_file_log(level: &str, message: &str) {
    if let Some(logger) = FILE_LOGGER.get() {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&logger.log_path)
        {
            use std::io::Write;
            let _ = writeln!(file, "[{}] {}", level, message);
        }
    }
}


