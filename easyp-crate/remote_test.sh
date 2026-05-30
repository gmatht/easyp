#!/bin/bash

# Deploy script for easyp HTTPS server
# Usage: ./deploy.sh <target_host>

set -e  # Exit on any error

# Function to tail server log on failures
tail_server_log() {
    echo "DEBUG: === SERVER LOG (last 50 lines) ==="
    ssh root@$SRV "tail -n 50 server.log 2>/dev/null || echo 'DEBUG: No server log found'"
    echo "DEBUG: === END SERVER LOG ==="
}

STAGING=--staging
#STAGING=
PROFILE=lto
		  
KEEPALIVE=y

if [ "$1" = quitafter ]
then
	KEEPALIVE=
	shift
fi

if [ -z "$1" ]
then
	SRV=$(cat .remote)
else
	SRV="$1"
fi

echo "DEBUG: Target server is $SRV"
echo "DEBUG: Starting deployment process..."

source ~/.cargo/env

# Build if needed
[ -f target/debug/easyp ] || RUSTC_WRAPPER= cargo build --bin easyp
if [ -z "$(find src/ */src/ -type f -newer target/debug/easyp 2>/dev/null)" ] || RUSTC_WRAPPER= cargo build --bin easyp --profile $PROFILE
then
	echo "DEBUG: Building completed, starting deployment..."
	
	echo "DEBUG: Killing existing easyp processes on remote server..."
	ssh root@$SRV "pkill easyp;sleep 1;pkill -9 easyp; true" && echo "DEBUG: Process cleanup completed"
	
	echo "DEBUG: Ensuring certificate directories are properly separated..."
	ssh root@$SRV "mkdir -p /var/lib/easyp/certs/staging /var/lib/easyp/certs/production" && echo "DEBUG: Certificate directories prepared"
	
	echo "DEBUG: Syncing binary to remote server..."
	rsync -avz ../target/$PROFILE/easyp root@$SRV: && echo "DEBUG: Binary sync completed"
	
	echo "DEBUG: Starting server in background..."
	echo "DEBUG: Using staging flag: [$STAGING]"
	ssh root@$SRV "pkill easyp; chmod +x easyp; nohup ./easyp --root /var/www/html $STAGING $VERBOSE $BOGUS > server.log 2>&1 &"
	echo "DEBUG: Server startup command sent to remote server"
	
	echo "DEBUG: Waiting 10 seconds for server to initialize..."
	sleep 10
	
	echo "DEBUG: Checking if server process is running on remote server..."
	if ssh root@$SRV "pgrep easyp > /dev/null"; then
		echo "DEBUG: Server process is running on remote server"
		echo "DEBUG: Checking server logs for startup completion..."
		ssh root@$SRV "tail -5 server.log"
	else
		echo "DEBUG: ERROR - Server process not found on remote server!"
		echo "DEBUG: Checking server logs..."
		ssh root@$SRV "tail -20 server.log" || echo "DEBUG: No server log found"
		exit 1
	fi
	
	echo "DEBUG: Testing server connectivity..."
	echo "DEBUG: Checking if port 80 is open..."
	if timeout 5 bash -c "echo > /dev/tcp/$SRV/80" 2>/dev/null; then
		echo "DEBUG: Port 80 is open"
	else
		echo "DEBUG: WARNING - Port 80 is not accessible"
	fi
	
	echo "DEBUG: Checking if port 443 is open..."
	if timeout 5 bash -c "echo > /dev/tcp/$SRV/443" 2>/dev/null; then
		echo "DEBUG: Port 443 is open"
	else
		echo "DEBUG: WARNING - Port 443 is not accessible"
	fi
	
	echo "DEBUG: Starting HTTP test with 10 second timeout..."
	echo === HTTP TEST ===
	if timeout 10 curl -v --connect-timeout 5 --max-time 10 "http://$SRV"; then
		echo "DEBUG: HTTP test completed successfully"
	else
		echo "ERROR: HTTP test failed or timed out"
		tail_server_log
		exit 1
	fi
	
	sleep 1
	
	echo "DEBUG: Starting HTTPS test with 15 second timeout..."
	echo === HTTPS TEST ===
	if timeout 15 curl -v --connect-timeout 10 --max-time 15 \
		--tlsv1.2 --tlsv1.3 \
		--ciphers 'ECDHE+AESGCM:ECDHE+CHACHA20:DHE+AESGCM:DHE+CHACHA20:!aNULL:!MD5:!DSS' \
		--retry 2 --retry-delay 1 \
		-k "https://$SRV"; then
		echo "DEBUG: HTTPS test completed successfully"
	else
		echo "ERROR: HTTPS test failed or timed out"
		echo "DEBUG: Attempting HTTPS test with different SSL options..."
		if timeout 15 curl -v --connect-timeout 10 --max-time 15 \
			--tls-max 1.3 --tlsv1.2 \
			--insecure --retry 1 \
			"https://$SRV" > /tmp/curl_output.log 2>&1; then
			echo "DEBUG: HTTPS test with fallback options completed"
		else
			echo "ERROR: HTTPS test failed with all SSL options"
			echo "DEBUG: Curl output:"
			cat /tmp/curl_output.log
			tail_server_log
			exit 1
		fi
	fi
	
	sleep 1
	
	echo "DEBUG: Testing certificate stability..."
	echo === CERTIFICATE STABILITY TEST ===
	
	# Function to get certificate fingerprint
	get_cert_fingerprint() {
		local port=$1
		timeout 15 openssl s_client -connect $SRV:$port -servername $SRV < /dev/null 2>/dev/null | openssl x509 -fingerprint -sha256 -noout 2>/dev/null | cut -d= -f2
	}
	
	# Determine which port to test based on server configuration
	# Check if server is running as root (port 443) or non-root (port 9443)
	HTTPS_PORT=443
	if ! timeout 5 bash -c "echo > /dev/tcp/$SRV/443" 2>/dev/null; then
		if timeout 5 bash -c "echo > /dev/tcp/$SRV/9443" 2>/dev/null; then
			HTTPS_PORT=9443
			echo "DEBUG: Using port 9443 (non-root user detected)"
		else
			echo "ERROR: Neither port 443 nor 9443 is accessible"
			tail_server_log
			exit 1
		fi
	else
		echo "DEBUG: Using port 443 (root user detected)"
	fi
	
	# Test 1: Certificate stability within same session
	echo "DEBUG: Testing certificate stability within same session..."
	CERT1=$(get_cert_fingerprint $HTTPS_PORT)
	sleep 3
	CERT2=$(get_cert_fingerprint $HTTPS_PORT)
	
	if [ -z "$CERT1" ] || [ -z "$CERT2" ]; then
		echo "ERROR: Certificate fingerprint test failed - could not retrieve certificates"
		tail_server_log
		exit 1
	fi
	
	if [ "$CERT1" != "$CERT2" ]; then
		echo "ERROR: Certificate stability test failed - certificate changed between requests"
		echo "   First cert:  $CERT1"
		echo "   Second cert: $CERT2"
		tail_server_log
		exit 1
	else
		echo "DEBUG: Certificate stability test passed - same certificate on multiple requests"
	fi
	
	# Test 2: Certificate stability after server restart
	echo "DEBUG: Testing certificate stability after server restart..."
	echo "DEBUG: Restarting server..."
	ssh root@$SRV "pkill easyp; sleep 2; nohup ./easyp --root /var/www/html $STAGING $VERBOSE $BOGUS > server.log 2>&1 &"
	echo "DEBUG: Waiting 10 seconds for server to restart..."
	sleep 10
	
	# Check if server is running after restart
	if ! ssh root@$SRV "pgrep easyp > /dev/null"; then
		echo "ERROR: Server failed to restart"
		tail_server_log
		exit 1
	fi
	
	# Get certificate after restart
	CERT3=$(get_cert_fingerprint $HTTPS_PORT)
	
	if [ -z "$CERT3" ]; then
		echo "ERROR: Could not retrieve certificate after restart"
		tail_server_log
		exit 1
	fi
	
	# Compare with original certificate
	if [ "$CERT1" != "$CERT3" ]; then
		echo "ERROR: Certificate changed after server restart"
		echo "   Original cert: $CERT1"
		echo "   After restart: $CERT3"
		tail_server_log
		exit 1
	else
		echo "DEBUG: Certificate stability after restart test passed - same certificate after restart"
	fi
	
	echo "DEBUG: Testing wget with security disabled..."
	echo === WGET TEST ===
	if ! timeout 20 wget --no-check-certificate --timeout=15 --tries=1 -q -O /tmp/wget_remote_test.html "https://$SRV" 2>/dev/null; then
		echo "ERROR: wget test failed"
		tail_server_log
		exit 1
	fi
	
	# Check if wget got non-empty content
	if [ ! -s /tmp/wget_remote_test.html ]; then
		echo "ERROR: wget test failed - received empty response"
		tail_server_log
		exit 1
	fi
	
	# Check if content looks like HTML
	if ! grep -q "<html\|<!DOCTYPE" /tmp/wget_remote_test.html 2>/dev/null; then
		echo "ERROR: wget test failed - response doesn't appear to be HTML"
		echo "   Response content:"
		head -5 /tmp/wget_remote_test.html
		tail_server_log
		exit 1
	fi
	
	echo "DEBUG: wget test passed - received valid HTML content"
	rm -f /tmp/wget_remote_test.html
	
	# Test 3: Non-root user certificate stability (if applicable)
	if [ "$HTTPS_PORT" = "9443" ]; then
		echo "DEBUG: Testing non-root user certificate stability..."
		echo === NON-ROOT USER CERTIFICATE TEST ===
		
		# Test with non-root user
		echo "DEBUG: Testing server as non-root user..."
		ssh root@$SRV "pkill easyp; sleep 2; sudo -u easytest nohup ./easyp --test-mode --verbose > server.log 2>&1 &"
		echo "DEBUG: Waiting 10 seconds for non-root server to start..."
		sleep 10
		
		# Check if non-root server is running
		if ! ssh root@$SRV "pgrep easyp > /dev/null"; then
			echo "ERROR: Non-root server failed to start"
			tail_server_log
			exit 1
		fi
		
		# Test certificate stability for non-root user
		CERT_NONROOT1=$(get_cert_fingerprint 9443)
		sleep 3
		CERT_NONROOT2=$(get_cert_fingerprint 9443)
		
		if [ -z "$CERT_NONROOT1" ] || [ -z "$CERT_NONROOT2" ]; then
			echo "ERROR: Non-root certificate fingerprint test failed"
			tail_server_log
			exit 1
		fi
		
		if [ "$CERT_NONROOT1" != "$CERT_NONROOT2" ]; then
			echo "ERROR: Non-root certificate stability test failed"
			echo "   First cert:  $CERT_NONROOT1"
			echo "   Second cert: $CERT_NONROOT2"
			tail_server_log
			exit 1
		else
			echo "DEBUG: Non-root certificate stability test passed"
		fi
		
		# Test non-root certificate persistence after restart
		echo "DEBUG: Testing non-root certificate persistence after restart..."
		ssh root@$SRV "pkill easyp; sleep 2; sudo -u easytest nohup ./easyp --test-mode --verbose > server.log 2>&1 &"
		sleep 10
		
		if ! ssh root@$SRV "pgrep easyp > /dev/null"; then
			echo "ERROR: Non-root server failed to restart"
			tail_server_log
			exit 1
		fi
		
		CERT_NONROOT3=$(get_cert_fingerprint 9443)
		if [ -z "$CERT_NONROOT3" ]; then
			echo "ERROR: Could not retrieve non-root certificate after restart"
			tail_server_log
			exit 1
		fi
		
		if [ "$CERT_NONROOT1" != "$CERT_NONROOT3" ]; then
			echo "ERROR: Non-root certificate changed after restart"
			echo "   Original cert: $CERT_NONROOT1"
			echo "   After restart: $CERT_NONROOT3"
			tail_server_log
			exit 1
		else
			echo "DEBUG: Non-root certificate persistence test passed"
		fi
		
		# Restart as root for cleanup
		echo "DEBUG: Restarting as root for cleanup..."
		ssh root@$SRV "pkill easyp; sleep 2; nohup ./easyp --root /var/www/html $STAGING $VERBOSE $BOGUS > server.log 2>&1 &"
		sleep 10
	fi
	
	echo === END TESTS ===
	
	if [ -z "$KEEPALIVE" ]
	then
		ssh root@$SRV "pkill easyp; sleep 1; pkill -9 easyp" || echo "DEBUG: Server process cleanup completed"
		echo "DEBUG: Stopping server process on remote server..."
	fi
	
	echo "DEBUG: Test script completed successfully"
	exit 0
fi
