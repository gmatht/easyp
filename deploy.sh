#!/bin/bash

# Deploy script for EasyPeas HTTPS server
# Usage: ./deploy.sh <target_host>

if [ $# -ne 1 ]; then
    echo "Usage: $0 <target_host>"
    echo "Example: $0 user@server.com"
    exit 1
fi

TARGET_HOST="$1"
BINARY_NAME="easyp"
SERVICE_NAME="easyp"

#echo "Building EasyP binary..."
#cargo build --release
#cross build --profile lto --target x86_64-unknown-linux-gnu

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

if [ "$(basename `pwd`)" = easyp-crate ]; then
	cd ..
fi



echo "Copying binary to $TARGET_HOST:/usr/local/bin/..."
scp target/x86_64-unknown-linux-gnu/lto/$BINARY_NAME $TARGET_HOST:/tmp/$BINARY_NAME

if [ $? -ne 0 ]; then
    echo "Failed to copy binary!"
    exit 1
fi

echo "Installing binary and setting up systemd service..."
ssh $TARGET_HOST << 'EOF'
    # Move binary to /usr/local/bin and set permissions
    sudo mv /tmp/easyp /usr/local/bin/easyp
    sudo chmod +x /usr/local/bin/easyp
    
    # Create necessary directories
    sudo mkdir -p /var/spool/easyp
    sudo mkdir -p /etc/letsencrypt
    sudo chown root:root /var/spool/easyp
    sudo chmod 755 /var/spool/easyp
    
    # Create comments directory structure
    sudo mkdir -p /var/spool/easyp/comments
    sudo chown www-data:www-data /var/spool/easyp/comments
    sudo chmod 755 /var/spool/easyp/comments
    
    # Create systemd service file
    sudo tee /etc/systemd/system/easyp.service > /dev/null << 'SERVICE_EOF'
[Unit]
Description=EasyP HTTPS Server
After=network.target

[Service]
Type=simple
User=root
Group=root
ExecStart=/usr/local/bin/easyp
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=false
PrivateTmp=true
ProtectSystem=false
ProtectHome=true
ReadWritePaths=/var/spool/easyp /var/www /etc/letsencrypt /var/lib/easyp

[Install]
WantedBy=multi-user.target
SERVICE_EOF

    # Reload systemd and enable service
    sudo systemctl daemon-reload
    sudo systemctl enable easyp.service
    
    # Restart the service to use the new binary
    sudo systemctl restart easyp.service
    
    echo "EasyPeas service installed, enabled, and restarted!"
    echo "To check status: sudo systemctl status easyp"
    echo "To view logs: sudo journalctl -u easyp -f"
EOF

if [ $? -eq 0 ]; then
    echo "Deployment completed successfully!"
else
    echo "Deployment failed!"
    exit 1
fi
