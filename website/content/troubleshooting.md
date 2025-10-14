---
title: "Troubleshooting"
description: "Common issues and solutions for easyp HTTPS server"
---

# Troubleshooting

## Common Issues

### 1. Permission Denied on Port 443
- Ensure running as root or with sudo
- Check if another service is using port 443

### 2. Certificate Not Found
- For ACME mode: Ensure domains are specified as command line arguments
- For legacy mode: Verify certificates exist in `/etc/letsencrypt/live/{domain}/`
- Check file permissions (should be readable by root)
- Ensure ACME_EMAIL environment variable is set for ACME mode

### 3. Admin Panel Not Accessible
- Check admin key in `/var/spool/easyp/admin`
  * or run `easyp --admin-urls` as root
- Verify URL format: `https://domain.com/extension_{key}`

### 4. Comments Not Appearing
- Check comment moderation in admin panel (at `easyp --admin-urls`)
- Verify `/var/spool/easyp/comments/` directory permissions
- Ensure `www-data` user has write access

### 5. Extensions Not Loading
- Check build output for compilation errors
- Verify extension files are in `extensions/` directory
- Ensure proper trait implementations

### 6. ACME Certificate Issues
- Verify domain is accessible from the internet
- Check that port 80 is open for HTTP-01 challenges
- Ensure ACME_EMAIL is set correctly
- Use staging environment first (`ACME_STAGING=true`)
- Check certificate directory permissions: `/var/lib/easyp/certs/`

## Debugging

- Check server logs: `sudo journalctl -u easyp -f`
- Verify file permissions: `ls -la /var/spool/easyp/comments/`
- Test admin access: `curl -k https://domain.com/comment_{key}`
- Check certificate validity: `openssl x509 -in /etc/letsencrypt/live/domain/fullchain.pem -text -noout`

## Testing

The project includes test scripts for different platforms:
- `setup_example.sh`: Sets up example domains and content (Linux/macOS)
- `test_server.sh`: Creates test environment with self-signed certificates (Linux/macOS)
- `test_server.bat`: Windows batch script for test environment setup
- `test_server.ps1`: PowerShell script for test environment setup

### Linux/macOS Testing
```bash
# Run the bash test script
chmod +x test_server.sh
./test_server.sh

# Test on a remote server
./remote_test.sh example.com

# Build and run the server
cargo build --release
sudo ./target/release/easyp
```

## Security Notes

- This is a basic implementation for development/testing
- In production, consider additional security measures
- Ensure proper file permissions on document roots
- Consider rate limiting and access controls




