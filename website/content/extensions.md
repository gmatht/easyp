---
title: "Extension System"
description: "Learn about easyp's powerful modular extension system"
---

**See Also**: [Spreadsheet Extension](/spreadsheet.html) - Interactive spreadsheet functionality

easyp features a powerful modular extension system with four types of extensions that allow you to customize and extend the server's functionality.

## Extension Types

1. **`.expand.rs`** - Content expansion extensions that modify HTML content
2. **`.bin.rs`** - CGI-bin like extensions for dynamic content generation
3. **`.root.rs`** - Root-level extensions that run before privilege dropping
4. **`.admin.rs`** - Admin panel extensions for content management

Drop these files into `extensions/` at compile time to have your extensions linked into your single file webserver.

## Built-in Example Extensions

### Comment System (`comment.*`)

- **`comment.expand.rs`**: Adds comment forms and displays live comments
- **`comment.bin.rs`**: Handles comment submission via CGI-like API
- **`comment.root.rs`**: Sets up comment directories and permissions
- **`comment.admin.rs`**: Provides comment moderation interface

#### Features
- **Comment Forms**: Automatically replaces `#EXTEND:comment()` in served html files
- **Live Comments**: Accepted comments appear immediately on the page
- **Moderation**: Admin interface for approving/rejecting comments
- **Security**: Comments are sanitized and validated
- **Storage**: Comments stored in `/var/spool/easyp/comments/`

#### Admin Panel
- Access via secret URL: `https://your-domain.com/comment_{admin_key}`
- Admin key is generated automatically on first run and stored in /var/spool/easyp.admin
- Batch moderation with checkboxes

### Math Extension (`math.expand.rs`)

- Converts `#EXTEND:hash()EXTEND:math(op,i,j)` blocks to rendered math, where op can be e.g. "add"

### Hash Extension (`hash.expand.rs`)

- Outputs a literal `#` character to escape the EXTEND directive syntax
- Useful for preventing unwanted expansion of extension directives
- Example: `#EXTEND:hash()EXTEND:hash()EXTEND:comment()` outputs `#EXTEND:hash()EXTEND:comment()` (which won't be expanded)

### Example Extension (`example.expand.rs`)

- Demonstrates basic extension functionality
- Adds example content to pages

### Upload Extension (`upload.*`)

The upload extension provides secure file upload and management capabilities.

#### Features

- **File Upload**: Upload files through a web interface
- **File Management**: List, view, and delete uploaded files
- **Security**: Files stored in `/var/www/html/uploads/` directory
- **Admin Interface**: Web-based admin panel for file management

#### Admin Panel

- Access via secret URL: `https://your-domain.com/upload_{admin_key}`
- Upload files via the web interface
- View uploaded files with metadata (size, modification time)
- Delete files through the admin interface

#### Storage

- Files stored in `/var/www/html/uploads/`
- Supports any file type
- Automatic directory creation
- Accessible via direct URL: `https://your-domain.com/uploads/{filename}`

### Commentloader Extension (`commentloader.bin.rs`)

The commentloader extension provides dynamic comment loading for improved caching performance.

#### Features

- **Uncached Endpoint**: Returns comments with `no-cache` headers
- **Performance**: Allows main page to be fully cached
- **Dynamic Loading**: Comments loaded via JavaScript iframe
- **Graceful Fallback**: Works even if JavaScript is disabled

#### API Endpoints

Available at `/cgi-bin/commentloader?return_url={url}`

**Purpose**: Load comments dynamically without caching, while allowing the main page HTML to be cached indefinitely.

**Implementation**: The comment extension (`comment.expand.rs`) creates an iframe that loads from this endpoint. JavaScript extracts the content from the iframe and integrates it into the page, providing seamless comment integration with optimal caching.

### Stats Extension (`stats.*`)

The stats extension provides server statistics and monitoring.

#### Features

- **Request Statistics**: Track HTTP/HTTPS request counts
- **Performance Monitoring**: Monitor server performance metrics
- **Real-Time Data**: Live statistics updated continuously
- **Admin Interface**: Web-based admin panel for viewing stats

#### Admin Panel

- Access via secret URL: `https://your-domain.com/stats_{admin_key}`
- View real-time request statistics
- Monitor server performance
- Export statistics data

### Logs Admin Extension (`logs.admin.rs`)

The logs admin extension provides log file viewing and management.

#### Features

- **Log Viewing**: View server logs through web interface
- **Log Rotation**: Configure log rotation settings
- **Search**: Search through log entries
- **Tail**: Real-time log tailing

#### Admin Panel

- Access via secret URL: `https://your-domain.com/logs_{admin_key}`
- View error and access logs
- Configure log settings
- Download log files

### All Admin Extension (`all.admin.rs`)

The all admin extension provides a central dashboard for all admin panels.

#### Features

- **Central Dashboard**: Access all admin panels from one place
- **Admin Key Display**: View all available admin keys
- **Quick Links**: Direct links to each admin panel
- **System Overview**: Quick view of all systems

#### Admin Panel

- Access via secret URL: `https://your-domain.com/all_{admin_key}`
- Central hub for all admin functionality
- Links to comment, stats, logs, upload, and other admin panels

### About Admin Extension (`about.admin.rs`)

The about admin extension provides information about the easyp server.

#### Features

- **Server Information**: Display server version and configuration
- **System Details**: Show system information
- **License**: Display license information
- **Links**: Links to documentation and resources

#### Admin Panel

- Access via secret URL: `https://your-domain.com/about_{admin_key}`
- Server information and documentation

### Worm Bin Extension (`worm.bin.rs`)

The worm bin extension provides secure, append-only storage for collaborative applications.

#### Features

- **Append/Create Only**: Files can only be appended to or created, never overwritten
- **Atomic Operations**: All append operations are atomic using file locking
- **Security**: Path traversal protection and restricted to worm/ directory only
- **Timestamped Entries**: All data is automatically timestamped
- **File Management**: List files, get file information, and read with range requests
- **Real-Time Monitoring**: Stream updates via SSE (Server-Sent Events)

#### API Endpoints

Available at `/cgi-bin/worm` with the following actions:

**Append Data** (`action=append`): Atomically append data to a file
**List Files** (`action=list`): List all files in the worm directory
**File Info** (`action=info`): Get file size and modification time
**Read** (`action=read`): Read file content with byte range support
**Stream** (`action=stream`): Real-time file updates via SSE (not fully implemented)

#### Security Features

- Path traversal protection prevents `../` and other malicious paths
- Directory restriction ensures all operations stay within `/worm/`
- Filename validation rejects invalid characters
- File locking ensures data integrity and thread safety

#### Worm Spreadsheet Application

The worm extension powers a real-time collaborative spreadsheet application demonstrating the append-only storage model.

**Features:**
- **Real-Time Collaboration**: Multiple users can edit simultaneously
- **Automatic Calculations**: Row sums, column sums, and global totals
- **Export Support**: Download data in .log, .csv, or .json formats
- **Append-Only Storage**: Complete audit trail of all changes
- **Offline Support**: Works offline with local storage fallback

**Try it:** [https://www.easyp.net/spreadsheet.html](https://www.easyp.net/spreadsheet.html)

The spreadsheet uses the worm bin extension to store data in an append-only format. Each cell edit is appended to a log file with a timestamp, creating a complete audit trail. The browser automatically polls for changes and updates the spreadsheet in real-time.

## Creating Custom Extensions

Extensions are automatically discovered by the build system. To create a new extension:

1. Add a `.rs` file to the `extensions/` directory with the appropriate suffix
2. Implement the required trait methods
3. The build system will automatically compile and register your extension

### Example: Custom Expand Extension

```rust
// extensions/my_extension.expand.rs
use std::collections::HashMap;

pub fn extend(url: &str, args: &str) -> String {
    // Your extension logic here
    format!("<div>Custom content for {}</div>", url)
}
```

## Admin System

easyp provides secure admin panels for content management:

### Admin Key Management
- Keys are generated dynamically on first run
- Stored in `/var/spool/easyp/admin`
- Keys are cached in memory for security
- Each extension gets its own unique admin key
- Go to https://example.com/KEY to administer system

### Security Features
- Admin keys are long, random alphanumeric strings
- Admin panels only accessible with correct keys
- Privilege dropping ensures admin operations run as `www-data`









