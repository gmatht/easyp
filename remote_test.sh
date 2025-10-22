#!/bin/bash

# Deploy script for easyp HTTPS server
# Usage: ./deploy.sh <target_host>

set -e  # Exit on any error

# Function to tail server logs when tests fail
tail_server_log() {
    echo "DEBUG: === SERVER LOG (last 50 lines) ==="
    ssh root@$SRV "tail -n 50 server.log 2>/dev/null || echo 'DEBUG: No server log found'"
    echo "DEBUG: === END SERVER LOG ==="
}

STAGING=--staging
STAGING=

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
if [ -z "$(find src/ */src easyp-crate/extensions -type f -newer target/debug/easyp 2>/dev/null)" ] || RUSTC_WRAPPER= cargo build --bin easyp
then
	echo "DEBUG: Building completed, starting deployment..."
	
	echo "DEBUG: Killing existing easyp processes on remote server..."
	ssh root@$SRV 'pkill easyp;sleep 1;pkill -9 easyp; rm /var/lib/easyp/certs/staging/*; true' && echo "DEBUG: Process cleanup completed"
	
	echo "DEBUG: Syncing binary to remote server..."
	rsync -avz target/debug/easyp root@$SRV: && echo "DEBUG: Binary sync completed"
	
	echo "DEBUG: Starting server in background..."
	ssh root@$SRV "pkill easyp; chmod +x easyp; nohup ./easyp --root /var/www/html $VERBOSE $STAGING $BOGUS > server.log 2>&1 &"
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
	if time bash -c "echo > /dev/tcp/$SRV/80" 2>/dev/null; then
		echo "DEBUG: Port 80 is open"
	else
		echo "DEBUG: WARNING - Port 80 is not accessible"
	fi
	
	echo "DEBUG: Checking if port 443 is open..."
	if time bash -c "echo > /dev/tcp/$SRV/443" 2>/dev/null; then
		echo "DEBUG: Port 443 is open"
	else
		echo "DEBUG: WARNING - Port 443 is not accessible"
	fi
	
	echo "DEBUG: Starting HTTP test with 10 second timeout..."
	echo === HTTP TEST ===
	if time curl -v --connect-timeout 5 --max-time 10 "http://$SRV"; then
		echo "DEBUG: HTTP test completed successfully"
	else
		echo "DEBUG: HTTP test failed or timed out"
		tail_server_log
	fi
	
	sleep 1
	
	echo "DEBUG: Starting HTTPS test with 60 second timeout..."
	echo === HTTPS TEST ===
	if time curl -v --connect-timeout 60 --max-time 60 -k "https://$SRV"; then
		echo "DEBUG: HTTPS test completed successfully"
	else
		echo "DEBUG: HTTPS test failed or timed out"
		tail_server_log
	fi
	
	sleep 1
	
	echo "DEBUG: Testing PNG file caching headers..."
	echo === PNG CACHE TEST ===
	if time curl -I "http://$SRV/easyp.png"; then
		echo "DEBUG: PNG cache test completed successfully"
	else
		echo "DEBUG: PNG cache test failed or timed out"
		tail_server_log
	fi
	
	echo "DEBUG: Testing admin extensions detection..."
	echo === ADMIN EXTENSIONS TEST ===
	
	# Test 1: Check if 'comment' extension appears in admin URLs (runtime detection)
	echo "DEBUG: Test 1 - Checking if 'comment' extension is automatically detected at runtime..."
	if ssh root@$SRV "./easyp --admin-urls" | grep -q "comment_"; then
		echo "DEBUG: ✅ SUCCESS - 'comment' extension detected in admin URLs at runtime"
		RUNTIME_TEST_PASSED=true
	else
		echo "DEBUG: ❌ FAILED - 'comment' extension not found in admin URLs at runtime"
		RUNTIME_TEST_PASSED=false
	fi
	
	# Test 2: Check that 'comment' is NOT hardcoded in easyp.rs (should be dynamically discovered)
	echo "DEBUG: Test 2 - Verifying 'comment' extension is NOT hardcoded in easyp.rs..."
	if grep -q '"comment"' easyp-crate/src/bin/easyp.rs; then
		echo "DEBUG: ⚠️  WARNING - 'comment' extension appears to be hardcoded in easyp.rs"
		echo "DEBUG: This suggests the extension system may not be fully dynamic"
		HARDCODED_IN_EASYP=true
	else
		echo "DEBUG: ✅ SUCCESS - 'comment' extension is not hardcoded in easyp.rs"
		HARDCODED_IN_EASYP=false
	fi
	
	# Test 3: Check that 'comment' is NOT hardcoded in build.rs
	echo "DEBUG: Test 3 - Verifying 'comment' extension is NOT hardcoded in build.rs..."
	if grep -q '"comment"' easyp-crate/build.rs; then
		echo "DEBUG: ⚠️  WARNING - 'comment' extension appears to be hardcoded in build.rs"
		echo "DEBUG: This suggests the build system may not be fully dynamic"
		HARDCODED_IN_BUILD=true
	else
		echo "DEBUG: ✅ SUCCESS - 'comment' extension is not hardcoded in build.rs"
		HARDCODED_IN_BUILD=false
	fi
	
	# Test 4: Check that build.rs dynamically discovers extensions
	echo "DEBUG: Test 4 - Verifying build.rs uses dynamic extension discovery..."
	if grep -q "read_dir.*extensions" easyp-crate/build.rs; then
		echo "DEBUG: ✅ SUCCESS - build.rs uses dynamic extension discovery (read_dir)"
		DYNAMIC_DISCOVERY=true
	else
		echo "DEBUG: ❌ FAILED - build.rs does not appear to use dynamic extension discovery"
		DYNAMIC_DISCOVERY=false
	fi
	
	# Test 5: Check that build.rs looks for .admin.rs files specifically
	echo "DEBUG: Test 5 - Verifying build.rs looks for .admin.rs files..."
	if grep -q "\.admin\.rs" easyp-crate/build.rs; then
		echo "DEBUG: ✅ SUCCESS - build.rs looks for .admin.rs files"
		ADMIN_RS_DETECTION=true
	else
		echo "DEBUG: ❌ FAILED - build.rs does not appear to look for .admin.rs files"
		ADMIN_RS_DETECTION=false
	fi
	
	# Summary
	echo "DEBUG: === EXTENSION SYSTEM TEST SUMMARY ==="
	if [ "$RUNTIME_TEST_PASSED" = true ] && [ "$HARDCODED_IN_EASYP" = false ] && [ "$HARDCODED_IN_BUILD" = false ] && [ "$DYNAMIC_DISCOVERY" = true ] && [ "$ADMIN_RS_DETECTION" = true ]; then
		echo "DEBUG: 🎉 ALL TESTS PASSED - Extension system is working correctly and is not hardcoded"
		echo "DEBUG: The 'comment' extension is automatically detected and integrated without hardcoding"
		echo "DEBUG: The system uses dynamic discovery of .admin.rs files"
	else
		echo "DEBUG: ⚠️  SOME TESTS FAILED - Extension system may have issues"
		echo "DEBUG: Runtime detection: $RUNTIME_TEST_PASSED"
		echo "DEBUG: Not hardcoded in easyp.rs: $HARDCODED_IN_EASYP"
		echo "DEBUG: Not hardcoded in build.rs: $HARDCODED_IN_BUILD"
		echo "DEBUG: Dynamic discovery: $DYNAMIC_DISCOVERY"
		echo "DEBUG: .admin.rs detection: $ADMIN_RS_DETECTION"
		tail_server_log
	fi
	
	echo === END TESTS ===
	
	echo "DEBUG: Testing non-root user functionality..."
	echo === NON-ROOT USER TEST ===
	
	# Create a test user if it doesn't exist
	echo "DEBUG: Creating test user 'easytest' if it doesn't exist..."
	ssh root@$SRV "id easytest >/dev/null 2>&1 || useradd -m easytest" || echo "DEBUG: User creation completed or user already exists"
	
	# Copy the binary to the test user's home directory
	echo "DEBUG: Copying easyp binary to test user's home directory..."
	ssh root@$SRV "cp easyp /home/easytest/ && chown easytest:easytest /home/easytest/easyp && chmod +x /home/easytest/easyp"
	
	# Test running as non-root user (should auto-enable --over-9000)
	echo "DEBUG: Testing easyp as non-root user (should auto-enable --over-9000)..."
	if ssh root@$SRV "sudo -u easytest /home/easytest/easyp --test-mode --admin-urls" 2>&1 | grep -q "Automatically enabling --over-9000"; then
		echo "DEBUG: ✅ SUCCESS - Non-root user correctly auto-enabled --over-9000"
		NON_ROOT_FALLBACK_TEST=true
	else
		echo "DEBUG: ❌ FAILED - Non-root user test did not show expected auto --over-9000 behavior"
		NON_ROOT_FALLBACK_TEST=false
	fi
	
	# Test that the server actually starts and responds
	echo "DEBUG: Testing that easyp starts successfully as non-root user..."
	if timeout 10 ssh root@$SRV "sudo -u easytest /home/easytest/easyp --test-mode > /tmp/nonroot_test.log 2>&1 &" && sleep 3; then
		if ssh root@$SRV "pgrep -f 'easyp.*test-mode' > /dev/null"; then
			echo "DEBUG: ✅ SUCCESS - Non-root user easyp process is running"
			NON_ROOT_STARTUP_TEST=true
			
			# Check the logs for auto --over-9000 messages
			if ssh root@$SRV "grep -q 'Automatically enabling --over-9000' /tmp/nonroot_test.log"; then
				echo "DEBUG: ✅ SUCCESS - Auto --over-9000 messages found in logs"
				NON_ROOT_LOG_TEST=true
			else
				echo "DEBUG: ⚠️  WARNING - Auto --over-9000 messages not found in logs"
				NON_ROOT_LOG_TEST=false
			fi
			
			# Test that the server actually responds to HTTP requests
			echo "DEBUG: Testing HTTP response from non-root server..."
			if ssh root@$SRV "curl -s -o /dev/null -w '%{http_code}' http://localhost:9080/ 2>/dev/null | grep -q '200\|404'"; then
				echo "DEBUG: ✅ SUCCESS - Server responds to HTTP requests on port 9080"
				NON_ROOT_HTTP_TEST=true
			else
				echo "DEBUG: ❌ FAILED - Server does not respond to HTTP requests"
				tail_server_log
				NON_ROOT_HTTP_TEST=false
			fi
			
			# Test HTTPS response (should work with self-signed certs in test mode)
			echo "DEBUG: Testing HTTPS response from non-root server..."
			if ssh root@$SRV "curl -s -k -o /dev/null -w '%{http_code}' https://localhost:9443/ 2>/dev/null | grep -q '200\|404'"; then
				echo "DEBUG: ✅ SUCCESS - Server responds to HTTPS requests on port 9443"
				NON_ROOT_HTTPS_TEST=true
			else
				echo "DEBUG: ❌ FAILED - Server does not respond to HTTPS requests"
				tail_server_log
				NON_ROOT_HTTPS_TEST=false
			fi
		else
			echo "DEBUG: ❌ FAILED - Non-root user easyp process is not running"
			tail_server_log
			NON_ROOT_STARTUP_TEST=false
			NON_ROOT_LOG_TEST=false
			NON_ROOT_HTTP_TEST=false
			NON_ROOT_HTTPS_TEST=false
		fi
		# Clean up the test process
		ssh root@$SRV "pkill -f 'easyp.*test-mode' || true"
	else
		echo "DEBUG: ❌ FAILED - Non-root user easyp failed to start"
		tail_server_log
		NON_ROOT_STARTUP_TEST=false
		NON_ROOT_LOG_TEST=false
		NON_ROOT_HTTP_TEST=false
		NON_ROOT_HTTPS_TEST=false
	fi
	
	# Test that ACME warnings are shown for non-root users
	echo "DEBUG: Testing ACME warnings for non-root users..."
	if ssh root@$SRV "sudo -u easytest /home/easytest/easyp --help" 2>&1 | grep -q "Non-root usage:"; then
		echo "DEBUG: ✅ SUCCESS - Help text includes non-root usage information"
		NON_ROOT_HELP_TEST=true
	else
		echo "DEBUG: ❌ FAILED - Help text does not include non-root usage information"
		NON_ROOT_HELP_TEST=false
	fi
	
	# Summary
	echo "DEBUG: === NON-ROOT USER TEST SUMMARY ==="
	if [ "$NON_ROOT_FALLBACK_TEST" = true ] && [ "$NON_ROOT_STARTUP_TEST" = true ] && [ "$NON_ROOT_LOG_TEST" = true ] && [ "$NON_ROOT_HTTP_TEST" = true ] && [ "$NON_ROOT_HTTPS_TEST" = true ] && [ "$NON_ROOT_HELP_TEST" = true ]; then
		echo "DEBUG: 🎉 ALL NON-ROOT TESTS PASSED - Non-root user functionality is working correctly"
		echo "DEBUG: - Auto --over-9000 detection: ✅"
		echo "DEBUG: - Server startup: ✅"
		echo "DEBUG: - Log messages: ✅"
		echo "DEBUG: - HTTP response (port 9080): ✅"
		echo "DEBUG: - HTTPS response (port 9443): ✅"
		echo "DEBUG: - Help text: ✅"
	else
		echo "DEBUG: ⚠️  SOME NON-ROOT TESTS FAILED"
		echo "DEBUG: - Auto --over-9000 detection: $NON_ROOT_FALLBACK_TEST"
		echo "DEBUG: - Server startup: $NON_ROOT_STARTUP_TEST"
		echo "DEBUG: - Log messages: $NON_ROOT_LOG_TEST"
		echo "DEBUG: - HTTP response: $NON_ROOT_HTTP_TEST"
		echo "DEBUG: - HTTPS response: $NON_ROOT_HTTPS_TEST"
		echo "DEBUG: - Help text: $NON_ROOT_HELP_TEST"
		tail_server_log
	fi
	
	# Clean up test user files
	echo "DEBUG: Cleaning up test user files..."
	ssh root@$SRV "rm -f /home/easytest/easyp /tmp/nonroot_test.log || true"
	
	echo === END NON-ROOT TESTS ===
	
	if [ -z "$KEEPALIVE" ]
	then
		ssh root@$SRV "pkill easyp; sleep 1; pkill -9 easyp" || echo "DEBUG: Server process cleanup completed"
		echo "DEBUG: Stopping server process on remote server..."
	fi
	
	echo "DEBUG: Test script completed"

	ssh root@$SRV "./easyp --admin-urls"
fi
