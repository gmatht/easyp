---
title: "Development"
description: "Development information and project structure for easyp"
---

## Project Structure

```
easyp/
├── easyp-crate/           # Main application source
│   ├── src/              # Source code
│   │   ├── bin/          # Binary executables
│   │   └── modules/      # Core modules
│   ├── extensions/       # Extension modules
│   │   ├── comment.*.rs  # Comment system extensions
│   │   ├── math.expand.rs # Math rendering extension
│   │   └── example.expand.rs # Example extension
│   └── Cargo.toml        # Main project configuration
├── rustls/               # Core TLS library
├── rustls-acme/          # ACME certificate management
├── acme-lib/             # ACME library
├── tokio-rustls/         # Async TLS support
├── website/              # Website files
└── target/               # Build output
    └── release/easyp     # Compiled binary
```

## Server Directory Structure

```
/var/www/{domain}/         # Document roots for each domain
/etc/letsencrypt/live/     # Let's Encrypt certificates
/var/spool/easyp/          # easyp data directory
├── admin                  # Admin keys file
└── comments/              # Comment system storage
    ├── in                 # Incoming comments
    ├── processing         # Comments awaiting moderation
    ├── accept             # Accepted comments
    ├── reject             # Rejected comments
    └── live/              # Live comments by URL hash
```

## Building

### Development Build
```bash
cargo build
```

### Release Build
```bash
cargo build --release
```

### LTO Build (Maximum Optimization)
```bash
cargo build --profile lto
```

## Extension Development

### Creating a New Extension

1. Create a new `.rs` file in `easyp-crate/extensions/` with the appropriate suffix:
   - `.expand.rs` for content expansion
   - `.bin.rs` for CGI-like functionality
   - `.root.rs` for root-level operations
   - `.admin.rs` for admin panels

2. Implement the required trait methods

3. The build system will automatically discover and compile your extension

### Example Extension Structure

```rust
// extensions/my_extension.expand.rs
use std::collections::HashMap;

pub fn extend(url: &str, args: &str) -> String {
    // Your extension logic here
    format!("<div>Custom content for {}</div>", url)
}
```

## Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
# Linux/macOS
./test_server.sh

# Windows
test_server.bat
```

### Manual Testing
```bash
# Build and run
cargo build --release
sudo ./target/release/easyp example.com
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

The easyp webserver is distributed under the GPLv3.

The library this was forked from was licensed under:
- Apache License version 2.0
- MIT license
- ISC license

The GPLv3 is liberal enough for what most normal people would want to do with a webserver, including most commercial purposes. If you want to distribute under a license other than GPLv3 feel free to drop us a line. Alternatively just use the permissively licensed upstream library at https://github.com/rustls/rustls

## TODO

- Security Audit
- Supply security updates via some secure channel
- Investigate feasibility of automatic free subdomain instead of self-signed cert fallback
- Fix warnings (Unused code: use or remove)
- Add more configuration options (e.g. aws-lc-ls without RSA)
- Sync with upstream
- Use Async/Await properly, don't fallback on polling
- Check that configuration options really work
- Detect file encoding (UTF8 vs UTF16 etc.) support unknown file types?









