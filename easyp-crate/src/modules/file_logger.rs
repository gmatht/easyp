// file_logger.rs - Persistent file logging system
// Writes logs to files with rotation and proper formatting

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// File logger that writes logs to a file
pub struct FileLogger {
    file: Arc<Mutex<File>>,
    log_path: String,
}

impl FileLogger {
    /// Create a new file logger
    pub fn new(log_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Ensure the directory exists
        if let Some(parent) = Path::new(log_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        Ok(FileLogger {
            file: Arc::new(Mutex::new(file)),
            log_path: log_path.to_string(),
        })
    }

    /// Write a log entry to the file
    pub fn write_log(&self, level: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let log_entry = format!("{} {}: {}\n", timestamp, level, message);

        let mut file = self.file.lock().unwrap();
        file.write_all(log_entry.as_bytes())?;
        file.flush()?;

        Ok(())
    }

    /// Get the log file path
    pub fn log_path(&self) -> &str {
        &self.log_path
    }

    /// Rotate the log file (rename current and create new)
    pub fn rotate(&self) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let rotated_path = format!("{}.{}", self.log_path, timestamp);

        // Close the current file
        drop(self.file.lock().unwrap());

        // Rename current file
        std::fs::rename(&self.log_path, &rotated_path)?;

        // Create new file
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        // Update the file reference
        *self.file.lock().unwrap() = new_file;

        Ok(())
    }
}

/// Global file logger instance
static FILE_LOGGER: OnceLock<Arc<FileLogger>> = OnceLock::new();

/// Initialize the global file logger
pub fn init_file_logger(log_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    FILE_LOGGER.set(Arc::new(FileLogger::new(log_path)?))
        .map_err(|_| "File logger already initialized")?;
    Ok(())
}

/// Write a log entry to the file logger
pub fn write_file_log(level: &str, message: &str) {
    if let Some(logger) = FILE_LOGGER.get() {
        if let Err(e) = logger.write_log(level, message) {
            eprintln!("Failed to write to file logger: {}", e);
        }
    }
}

/// Get the current log file path
pub fn get_log_file_path() -> Option<String> {
    FILE_LOGGER.get().map(|logger| logger.log_path().to_string())
}

/// Rotate the log file
pub fn rotate_log_file() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(logger) = FILE_LOGGER.get() {
        logger.rotate()?;
    }
    Ok(())
}

