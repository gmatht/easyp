//! Enhanced error reporting module
//! Provides detailed error information for file operations and network operations

use std::error::Error;
use std::fmt;
use std::io;
use std::path::Path;

/// Enhanced error type that provides context about what operation failed
#[derive(Debug)]
pub struct EnhancedError {
    pub operation: String,
    pub path: Option<String>,
    pub original_error: Box<dyn Error + Send + Sync>,
}

impl fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref path) = self.path {
            write!(f, "{} failed for '{}': {}", self.operation, path, self.original_error)
        } else {
            write!(f, "{} failed: {}", self.operation, self.original_error)
        }
    }
}

impl Error for EnhancedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.original_error.as_ref())
    }
}

/// Helper function to create enhanced error for file operations
pub fn file_operation_error<P: AsRef<Path>>(
    operation: &str,
    path: P,
    error: io::Error,
) -> EnhancedError {
    EnhancedError {
        operation: operation.to_string(),
        path: Some(path.as_ref().to_string_lossy().to_string()),
        original_error: Box::new(error),
    }
}

/// Helper function to create enhanced error for network operations
pub fn network_operation_error(
    operation: &str,
    address: &str,
    error: Box<dyn Error + Send + Sync>,
) -> EnhancedError {
    EnhancedError {
        operation: operation.to_string(),
        path: Some(address.to_string()),
        original_error: error,
    }
}

/// Helper function to create enhanced error for general operations
pub fn operation_error(
    operation: &str,
    error: Box<dyn Error + Send + Sync>,
) -> EnhancedError {
    EnhancedError {
        operation: operation.to_string(),
        path: None,
        original_error: error,
    }
}

/// Enhanced file operations with detailed error reporting
pub mod file_ops {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// Enhanced version of std::fs::create_dir_all with detailed error reporting
    pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<(), EnhancedError> {
        fs::create_dir_all(&path)
            .map_err(|e| file_operation_error("create_dir_all", &path, e))
    }

    /// Enhanced version of std::fs::write with detailed error reporting
    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<(), EnhancedError> {
        fs::write(&path, contents)
            .map_err(|e| file_operation_error("write", &path, e))
    }

    /// Enhanced version of std::fs::read_to_string with detailed error reporting
    pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, EnhancedError> {
        fs::read_to_string(&path)
            .map_err(|e| file_operation_error("read_to_string", &path, e))
    }

    /// Enhanced version of std::fs::metadata with detailed error reporting
    pub fn metadata<P: AsRef<Path>>(path: P) -> Result<fs::Metadata, EnhancedError> {
        fs::metadata(&path)
            .map_err(|e| file_operation_error("metadata", &path, e))
    }

    /// Enhanced version of std::fs::set_permissions with detailed error reporting
    pub fn set_permissions<P: AsRef<Path>>(path: P, perm: fs::Permissions) -> Result<(), EnhancedError> {
        fs::set_permissions(&path, perm)
            .map_err(|e| file_operation_error("set_permissions", &path, e))
    }

    /// Enhanced version of std::fs::read_dir with detailed error reporting
    pub fn read_dir<P: AsRef<Path>>(path: P) -> Result<fs::ReadDir, EnhancedError> {
        fs::read_dir(&path)
            .map_err(|e| file_operation_error("read_dir", &path, e))
    }

    /// Enhanced version of std::fs::rename with detailed error reporting
    pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<(), EnhancedError> {
        fs::rename(&from, &to)
            .map_err(|e| file_operation_error("rename", &from, e))
    }
}

/// Enhanced network operations with detailed error reporting
pub mod network_ops {
    use super::*;
    use tokio::net::TcpListener;

    /// Enhanced version of TcpListener::bind with detailed error reporting
    pub async fn bind_tcp_listener(addr: &str) -> Result<TcpListener, EnhancedError> {
        TcpListener::bind(addr).await
            .map_err(|e| network_operation_error("bind_tcp_listener", addr, Box::new(e)))
    }
}

/// Macro to wrap any Result with enhanced error reporting
#[macro_export]
macro_rules! enhanced_result {
    ($operation:expr, $path:expr, $result:expr) => {
        $result.map_err(|e| crate::modules::enhanced_error::file_operation_error($operation, $path, e))
    };
}

/// Macro to wrap network operations with enhanced error reporting
#[macro_export]
macro_rules! enhanced_network_result {
    ($operation:expr, $addr:expr, $result:expr) => {
        $result.map_err(|e| crate::modules::enhanced_error::network_operation_error($operation, $addr, Box::new(e)))
    };
}
