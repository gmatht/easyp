// stats.root.rs - Root setup for stats extension
// Handles directory setup and initialization for stats functionality

use std::fs;
use std::path::Path;

// Setup stats directories and files
pub fn setup_stats_directories() -> Result<(), String> {
    // Create stats data directory if it doesn't exist
    let stats_dir = Path::new("/var/lib/easyp/stats");

    if !stats_dir.exists() {
        fs::create_dir_all(stats_dir)
            .map_err(|e| format!("Failed to create stats directory: {}", e))?;

        // Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(stats_dir)
                .map_err(|e| format!("Failed to get metadata for stats directory: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(stats_dir, perms)
                .map_err(|e| format!("Failed to set permissions for stats directory: {}", e))?;
        }
    }

    // Create stats log directory if it doesn't exist
    let stats_log_dir = Path::new("/var/log/easyp/stats");

    if !stats_log_dir.exists() {
        fs::create_dir_all(stats_log_dir)
            .map_err(|e| format!("Failed to create stats log directory: {}", e))?;

        // Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(stats_log_dir)
                .map_err(|e| format!("Failed to get metadata for stats log directory: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(stats_log_dir, perms)
                .map_err(|e| format!("Failed to set permissions for stats log directory: {}", e))?;
        }
    }

    Ok(())
}

