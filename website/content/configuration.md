---
title: "Configuration"
description: "Configuration options and environment variables for easyp"
---

# Configuration

## ACME Configuration

The server supports two modes of operation:

**ACME Mode (Default when domains are specified):**
- Automatically requests Let's Encrypt certificates for specified domains
- Handles HTTP-01 challenges for domain validation
- Stores certificates in `/var/lib/easyp/certs/`
- Automatically renews certificates before expiration
- Uses staging environment by default (set `ACME_STAGING=false` for production)

**Legacy Mode (Fallback):**
- Scans `/etc/letsencrypt/live/` for existing domains
- Uses pre-existing certificates from certbot or other tools
- No automatic certificate management

## Environment Variables

- `ACME_EMAIL`: Email address for Let's Encrypt registration (defaults to `webmaster@$HOSTNAME` if hostname contains a dot, otherwise `webmaster@domain` where domain is the shortest domain found by reverse DNS, or `webmaster@localhost` as final fallback)
- `ACME_STAGING`: Set to "true" for staging Let's Encrypt environment (defaults to "false" for production)
- `ENABLE_DNS_DISCOVERY`: Enable automatic hostname discovery via DNS (defaults to "true")

## General Configuration

The server automatically:
- Serves files from `/var/www/{domain}/` for each domain
- Uses the first domain found as the default domain
- Maps file extensions to appropriate MIME types
- Handles ACME HTTP-01 challenges at `/.well-known/acme-challenge/`

## Systemd Service

The easyp service includes:
- Automatic restart on failure
- Security hardening (NoNewPrivileges, PrivateTmp, etc.)
- Proper file system access controls
- Journal logging

Service management:
```bash
sudo systemctl start easyp      # Start service
sudo systemctl stop easyp       # Stop service
sudo systemctl restart easyp    # Restart service
sudo systemctl status easyp     # Check status
sudo journalctl -u easyp -f     # View logs
```


## Performance Optimization

The project includes optimized build profiles:

- **Debug**: Fast compilation with debug info
- **"Release"**: Maximum optimization
- **LTO**: Maximum optimization with LTO, size optimization, and stripped symbols

### Binary Sizes
- Debug build: ~62 MB
- "Release" build: ~4.9 MB
- LTO build: ~2.8MB (used for release)

The LTO profile uses:
- `lto = "fat"`: Full Link Time Optimization
- `codegen-units = 1`: Single codegen unit for better optimization
- `opt-level = "z"`: Optimize for size
- `strip = true`: Remove debug symbols









