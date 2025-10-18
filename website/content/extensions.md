---
title: "Extension System"
description: "Learn about easyp's powerful modular extension system"
---

# Extension System

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
- **Comment Forms**: Automatically replaces '#EXPAND:comment()' in served html files
- **Live Comments**: Accepted comments appear immediately on the page
- **Moderation**: Admin interface for approving/rejecting comments
- **Security**: Comments are sanitized and validated
- **Storage**: Comments stored in `/var/spool/easyp/comments/`

#### Admin Panel
- Access via secret URL: `https://your-domain.com/comment_{admin_key}`
- Admin key is generated automatically on first run and stored in /var/spool/easyp.admin
- Batch moderation with checkboxes

### Math Extension (`math.expand.rs`)

- Converts `#EXPAND:math(op,i,j)` blocks to rendered math, where op can be e.g. "add"

### Example Extension (`example.expand.rs`)

- Demonstrates basic extension functionality
- Adds example content to pages

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









