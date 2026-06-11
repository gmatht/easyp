#!/bin/bash

# Deploy script for EasyPeas HTTPS server
# Usage: ./deploy.sh [-p|--production] <target_host>

PRODUCTION=false

while [[ "$1" =~ ^- ]]; do
    case "$1" in
        -p|--production)
            PRODUCTION=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

if [ $# -ne 1 ]; then
    echo "Usage: $0 [-p|--production] <target_host>"
    echo "Example: $0 user@server.com"
    echo "         $0 -p user@server.com"
    exit 1
fi

TARGET_HOST="$1"

BINARY_FLAGS=""
if [ "$PRODUCTION" = false ]; then
    BINARY_FLAGS="--staging"
fi

BINARY_NAME="easyp"
SERVICE_NAME="easyp"

# Locate existing binary in common target locations or attempt to build it.
# Fall back to target/release/ if that doesn't exist.

# Candidate paths to check (in order of preference)
possible_paths=(
    "target/x86_64-unknown-linux-gnu/release/$BINARY_NAME"
    "easyp-crate/target/x86_64-unknown-linux-gnu/release/$BINARY_NAME"
    "target/release/$BINARY_NAME"
    "easyp-crate/target/release/$BINARY_NAME"
    "target/$BINARY_NAME"
    "easyp-crate/target/$BINARY_NAME"
)

LOCAL_BINARY=""
for p in "${possible_paths[@]}"; do
    if [ -f "$p" ]; then
        LOCAL_BINARY="$p"
        break
    fi
done

if [ -z "$LOCAL_BINARY" ]; then
    echo "Binary not found in target directories. Attempting to build (this may take a while)..."

    # Prefer `cross` for reproducible targets if available, otherwise use cargo.
    if command -v cross >/dev/null 2>&1; then
        echo "Building with: cross build --release --target x86_64-unknown-linux-gnu"
        if ! cross build --release --target x86_64-unknown-linux-gnu; then
            echo "cross build failed, falling back to cargo build --release"
            if ! cargo build --release; then
                echo "cargo build --release failed!"
                exit 1
            fi
        fi
    else
        echo "Building with: cargo build --release"
        if ! cargo build --release; then
            echo "cargo build failed!"
            exit 1
        fi
    fi

    # re-check for the binary after build
    for p in "${possible_paths[@]}"; do
        if [ -f "$p" ]; then
            LOCAL_BINARY="$p"
            break
        fi
    done

    if [ -z "$LOCAL_BINARY" ]; then
        echo "Build completed but binary not found. Please build a portable binary or adjust the script to point to the correct path."
        exit 1
    fi
fi

if [ "$(basename `pwd`)" = easyp-crate ]; then
	cd ..
fi



echo "Copying binary ($LOCAL_BINARY) to $TARGET_HOST:/usr/local/bin/..."
scp "$LOCAL_BINARY" "$TARGET_HOST:/tmp/$BINARY_NAME"

if [ $? -ne 0 ]; then
    echo "Failed to copy binary!"
    exit 1
fi

echo "Installing binary and setting up systemd service..."
ssh $TARGET_HOST << EOF
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
ExecStart=/usr/local/bin/easyp $BINARY_FLAGS
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
