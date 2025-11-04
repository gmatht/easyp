---
title: "Installation"
description: "How to install and set up easyp HTTPS server"
---

## Quick Start

1. Clone or download this project
2. Build the server:
   ```bash
   cargo build --release
   ```
3. Deploy using the included script:
   ```bash
   ./deploy.sh user@your-server.com
   ```

## Requirements

- Let's Encrypt certificates in `/etc/letsencrypt/live/` (or use auto-ACME mode)
- Document roots in `/var/www/{domain}/` or common directory `/var/www/html`
- Root privileges to bind to port 80 and 443 (you can use other ports but auto-ACME require port 80)

## ACME Mode (Recommended)

1. Create document roots for your domains:
   ```bash
   sudo mkdir -p /var/www/example.com
   echo "<h1>Hello from example.com!</h1>" | sudo tee /var/www/example.com/index.html
   ```

2. Run the server with ACME certificate management:
   ```bash
   # Email defaults to webmaster@$HOSTNAME (if hostname contains a dot) or webmaster@domain (shortest domain from reverse DNS) or webmaster@localhost
   # Staging defaults to false (production Let's Encrypt)
   
   # Run with domain list
   sudo ./target/release/easyp example.com another-domain.com
   ```

3. For testing, use the staging environment:
   ```bash
   export ACME_STAGING="true"
   sudo ./target/release/easyp example.com another-domain.com
   ```

4. Customize the email address:
   ```bash
   export ACME_EMAIL="admin@example.com"
   sudo ./target/release/easyp example.com another-domain.com
   ```

## Legacy Mode (Let's Encrypt Directory)

1. Ensure your Let's Encrypt certificates are in place:
   ```
   /etc/letsencrypt/live/example.com/fullchain.pem
   /etc/letsencrypt/live/example.com/privkey.pem
   ```

2. Create document roots for your domains:
   ```bash
   sudo mkdir -p /var/www/example.com
   echo "<h1>Hello from example.com!</h1>" | sudo tee /var/www/example.com/index.html
   ```

3. Run the server (requires root for port 443):
   ```bash
   sudo ./target/release/easyp
   ```

   Or use the systemd service (if deployed):
   ```bash
   sudo systemctl start easyp
   sudo systemctl enable easyp  # Start on boot
   ```

4. Visit your domains:
   - http://example.com (HTTP on port 80)
   - https://example.com (HTTPS on port 443)
   - http://another-domain.com
   - https://another-domain.com

## Automated Deployment

Use the included `deploy.sh` script for easy deployment:

```bash
./deploy.sh user@your-server.com
```

This script will:
1. Build the release binary
2. Copy it to the target server
3. Install it to `/usr/local/bin/easyp`
4. Create and enable a systemd service
5. Set up proper security configurations

## Manual Deployment

1. Build the binary:
   ```bash
   cargo build --release
   ```

2. Copy to target server:
   ```bash
   scp target/release/easyp user@server:/usr/local/bin/
   ```

3. Set permissions:
   ```bash
   ssh user@server "sudo chmod +x /usr/local/bin/easyp"
   ```

4. Create systemd service (see `deploy.sh` for the service file)









