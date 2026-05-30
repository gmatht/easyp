// upload.root.rs - Root setup for upload extension
// Sets up upload directories and permissions

use std::fs;
use std::path::Path;

/// Set up upload directories and permissions
pub fn setup_upload_directories() -> Result<(), Box<dyn std::error::Error>> {
    let upload_dir = Path::new("/var/spool/easyp/uploads");

    // Create upload directory
    if !upload_dir.exists() {
        fs::create_dir_all(upload_dir)?;
        println!("Created upload directory: {}", upload_dir.display());
    }

    // Set permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(upload_dir)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(upload_dir, perms)?;
        println!("Set permissions for upload directory: {}", upload_dir.display());
    }

    // Create public uploads directory for serving files
    let public_uploads_dir = Path::new("/var/www/html/uploads");
    if !public_uploads_dir.exists() {
        fs::create_dir_all(public_uploads_dir)?;
        println!("Created public uploads directory: {}", public_uploads_dir.display());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(public_uploads_dir)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(public_uploads_dir, perms)?;
        }
    }

    // Create symlink from public uploads to private uploads (if not exists)
    #[cfg(unix)]
    {
        let symlink_path = Path::new("/var/www/html/uploads");
        if !symlink_path.exists() {
            // Remove the directory we just created and create a symlink instead
            if symlink_path.is_dir() {
                fs::remove_dir(symlink_path)?;
            }
            std::os::unix::fs::symlink(upload_dir, symlink_path)?;
            println!("Created symlink from {} to {}", symlink_path.display(), upload_dir.display());
        }
    }

    Ok(())
}


