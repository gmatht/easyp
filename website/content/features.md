---
title: "Features"
description: "Comprehensive list of easyp HTTPS server features"
---

easyp is a modern HTTPS server with automatic certificate management and extension support.

## Core Features

- **ACME Integration**: Built-in Let's Encrypt certificate management (no certbot required!)
- **Automatic Domain Discovery**: Scans `/etc/letsencrypt/live/` for domains or uses ACME
- **Dual Protocol Support**: Serves both HTTP (port 80) and HTTPS (port 443)
- **Static File Serving**: Serves files from `/var/www/{domain}/` for each domain
   * Falls back to /var/www/html
- **Let's Encrypt Integration**: Uses fullchain.pem and privkey.pem certificates
- **HTTP-01 Challenge Support**: Handles ACME domain validation challenges
- **Automatic Certificate Renewal**: Background task for certificate management

## Extension System

- **Extension System**: Modular architecture with multiple extension types
- **Comment System**: Example extensions implementing commenting with moderation capabilities
- **Admin Panels**: Secure admin interfaces for content management
- **Privilege Dropping**: Drops to `www-data` after initialization for security
- **CGI-like Support**: Executes statically compile and linked CGI-like scripts for dynamic content

## Technical Features

- **MIME Type Support**: Proper content types for common file formats
- **Comprehensive Logging**: Detailed request and error logging
- **Security Hardening**: Built-in security features and privilege management
- **Performance Optimization**: Optimized builds with LTO and size optimization

## Supported File Types

- HTML: `.html`
- CSS: `.css`
- JavaScript: `.js`
- JSON: `.json`
- WASM: `.wasm`
- Images: `.png`, `.jpg`, `.jpeg`, `.gif`, `.svg`, `.ico`
- Text: `.txt`
- Default: `application/octet-stream`









