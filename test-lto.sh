#!/bin/bash

# Function to test HTTPS server locally
test_https_server() {
    echo "🔍 Starting local server for testing..."
    
    # Kill any existing easyp processes
    pkill easyp 2>/dev/null || true
    sleep 1
    
    # Start server in background
    ./target/lto/easyp --verbose --test-mode --http-port 8080 --https-port 8443 > /tmp/easyp_test.log 2>&1 &
    SERVER_PID=$!
    
    # Wait for server to start
    echo "⏳ Waiting for server to start..."
    sleep 5
    
    # Check if server is running
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "❌ Server failed to start"
        echo "Server log:"
        cat /tmp/easyp_test.log
        return 1
    fi
    
    # Test HTTP
    echo "🔍 Testing HTTP connection..."
    if ! timeout 10 curl -s -f "http://localhost:8080" > /dev/null; then
        echo "❌ HTTP test failed"
        kill $SERVER_PID 2>/dev/null || true
        return 1
    fi
    
    # Test HTTPS
    echo "🔍 Testing HTTPS connection..."
    if ! timeout 10 curl -s -f -k "https://localhost:8443" > /dev/null; then
        echo "❌ HTTPS test failed"
        kill $SERVER_PID 2>/dev/null || true
        return 1
    fi
    
    # Test certificate stability
    echo "🔍 Testing certificate stability..."
    CERT1=$(timeout 10 openssl s_client -connect localhost:8443 -servername localhost < /dev/null 2>/dev/null | openssl x509 -fingerprint -sha256 -noout 2>/dev/null | cut -d= -f2)
    sleep 2
    CERT2=$(timeout 10 openssl s_client -connect localhost:8443 -servername localhost < /dev/null 2>/dev/null | openssl x509 -fingerprint -sha256 -noout 2>/dev/null | cut -d= -f2)
    
    if [ -z "$CERT1" ] || [ -z "$CERT2" ]; then
        echo "❌ Certificate fingerprint test failed - could not retrieve certificates"
        kill $SERVER_PID 2>/dev/null || true
        return 1
    fi
    
    if [ "$CERT1" != "$CERT2" ]; then
        echo "❌ Certificate stability test failed - certificate changed between requests"
        echo "   First cert:  $CERT1"
        echo "   Second cert: $CERT2"
        kill $SERVER_PID 2>/dev/null || true
        return 1
    else
        echo "✅ Certificate stability test passed - same certificate on multiple requests"
    fi
    
    # Test wget with security disabled
    echo "🔍 Testing wget with security disabled..."
    if ! timeout 15 wget --no-check-certificate --timeout=10 --tries=1 -q -O /tmp/wget_test.html "https://localhost:8443" 2>/dev/null; then
        echo "❌ wget test failed"
        kill $SERVER_PID 2>/dev/null || true
        return 1
    fi
    
    # Check if wget got non-empty content
    if [ ! -s /tmp/wget_test.html ]; then
        echo "❌ wget test failed - received empty response"
        kill $SERVER_PID 2>/dev/null || true
        return 1
    fi
    
    # Check if content looks like HTML
    if ! grep -q "<html\|<!DOCTYPE" /tmp/wget_test.html 2>/dev/null; then
        echo "❌ wget test failed - response doesn't appear to be HTML"
        echo "   Response content:"
        head -5 /tmp/wget_test.html
        kill $SERVER_PID 2>/dev/null || true
        return 1
    fi
    
    echo "✅ wget test passed - received valid HTML content"
    rm -f /tmp/wget_test.html
    
    # Clean up
    kill $SERVER_PID 2>/dev/null || true
    sleep 1
    pkill easyp 2>/dev/null || true
    
    return 0
}

# Function to test remote server
test_remote_server() {
    echo "🔍 Testing remote server..."
    
    # Check if remote test script exists
    if [ ! -f "./easyp-crate/remote_test.sh" ]; then
        echo "⚠️  Remote test script not found, skipping remote test"
        return 0
    fi
    
    # Check if .remote file exists
    if [ ! -f ".remote" ]; then
        echo "⚠️  .remote file not found, skipping remote test"
        return 0
    fi
    
    # Run remote test with quitafter flag
    if ./easyp-crate/remote_test.sh quitafter; then
        return 0
    else
        return 1
    fi
}

# Build the LTO binary
echo "🔨 Building LTO binary..."
cargo build --bin easyp --profile lto

# Check if build was successful
if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

# Get current binary size
CURRENT_SIZE=$(stat -c%s target/lto/easyp 2>/dev/null || echo "0")
CURRENT_SIZE_KB=$((CURRENT_SIZE / 1024))

# Size tracking file
SIZE_FILE=".lto_size"

# Check if we have a previous size recorded
if [ -f "$SIZE_FILE" ]; then
    PREVIOUS_SIZE=$(cat "$SIZE_FILE")
    PREVIOUS_SIZE_KB=$((PREVIOUS_SIZE / 1024))
    
    if [ "$CURRENT_SIZE" -lt "$PREVIOUS_SIZE" ]; then
        # Binary got smaller - good!
        SIZE_DIFF=$((PREVIOUS_SIZE - CURRENT_SIZE))
        SIZE_DIFF_KB=$((SIZE_DIFF / 1024))
        echo "✅ Binary shrunk: ${PREVIOUS_SIZE_KB}KB -> ${CURRENT_SIZE_KB}KB (saved ${SIZE_DIFF_KB}KB)"
        
        # Test the server before committing
        echo "🧪 Testing HTTPS server locally..."
        if test_https_server; then
            echo "✅ Local HTTPS test passed!"
            
            echo "🌐 Testing remote server..."
            if test_remote_server; then
                echo "✅ Remote server test passed!"
                echo "📝 Committing changes..."
                ./commita shrunk "${PREVIOUS_SIZE_KB}KB -> ${CURRENT_SIZE_KB}KB"
                # Update the size file
                echo "$CURRENT_SIZE" > "$SIZE_FILE"
            else
                echo "❌ Remote server test failed! Not committing."
                exit 1
            fi
        else
            echo "❌ Local HTTPS test failed! Not committing."
            exit 1
        fi
    elif [ "$CURRENT_SIZE" -gt "$PREVIOUS_SIZE" ]; then
        # Binary got larger - bloat warning!
        SIZE_DIFF=$((CURRENT_SIZE - PREVIOUS_SIZE))
        SIZE_DIFF_KB=$((SIZE_DIFF / 1024))
        echo "⚠️  BLOAT WARNING: Binary grew from ${PREVIOUS_SIZE_KB}KB to ${CURRENT_SIZE_KB}KB (+${SIZE_DIFF_KB}KB)"
        echo "🤔 REVERT YOUR LAST CHANGE????"
        # Don't update the size file - keep the smaller size as reference
    else
        # Same size
        echo "📏 Binary size unchanged: ${CURRENT_SIZE_KB}KB"
    fi
else
    # First time running - just record the size
    echo "📏 Initial binary size: ${CURRENT_SIZE_KB}KB"
    echo "$CURRENT_SIZE" > "$SIZE_FILE"
fi

# Show the binary info
echo ""
ls -l target/lto/easyp
