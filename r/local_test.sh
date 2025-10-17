#!/bin/bash

# Local test script for easyp HTTPS server (pure bash version)
# Usage: ./local_test.sh [quitafter]

#set -e  # Exit on any error

STAGING=--staging
#STAGING=
		  
KEEPALIVE=y

if [ "$1" = quitafter ]
then
	KEEPALIVE=
	shift
fi

# Use localhost for local testing
SRV="localhost"

echo "DEBUG: Target server is $SRV (localhost)"
echo "DEBUG: Starting local test process..."

# Build if needed
	mkdir -p /var/www/html
	
	# Add additional test files to verify correct serving (without touching index.html)
	echo "Test file content" > /var/www/html/test.txt
	echo "<html><body><h2>Test Page</h2><p>This is a test page in /var/www/html</p><link rel='stylesheet' href='/css/test.css'><script src='/js/test.js'></script></body></html>" > /var/www/html/test.html
	
	# Create additional test resources for wget -p to download
	mkdir -p /var/www/html/css /var/www/html/js
	echo "/* Test CSS file */ body { background-color: #f0f0f0; }" > /var/www/html/css/test.css
	echo "// Test JS file console.log('Test JS loaded');" > /var/www/html/js/test.js
	
	# Start the server with the correct web root
	ls /root
	ls /root/r
	#chmod +x /root/r/easyp
	/root/r/easyp --root /var/www/html --staging $VERBOSE $STAGING_FLAG $BOGUS 2>&1 | tee server.log &
	easypid=$!
	echo "DEBUG: Server startup command executed"
	
	echo "DEBUG: Waiting 10 seconds for server to initialize..."
	sleep 10
	
	echo "DEBUG: Testing server connectivity..."
	echo "DEBUG: Checking if port 80 is open..."
	if bash -c "echo > /dev/tcp/$SRV/80" 2>/dev/null; then
		echo "DEBUG: Port 80 is open"
	else
		echo "DEBUG: WARNING - Port 80 is not accessible"
	fi
	
	echo "DEBUG: Checking if port 443 is open..."
	if bash -c "echo > /dev/tcp/$SRV/443" 2>/dev/null; then
		echo "DEBUG: Port 443 is open"
	else
		echo "DEBUG: WARNING - Port 443 is not accessible"
	fi
	
	# Pure bash HTTP client function
	http_request() {
		local method="$1"
		local url="$2"
		local host="$3"
		local port="$4"
		local path="$5"
		local timeout="${6:-10}"
		
		# Parse URL if full URL provided
		if [[ "$url" =~ ^https?://([^/]+)(.*)$ ]]; then
			host="${BASH_REMATCH[1]}"
			path="${BASH_REMATCH[2]}"
			if [[ "$url" =~ ^https:// ]]; then
				port=443
			else
				port=80
			fi
		fi
		
		# Default values
		host="${host:-$SRV}"
		port="${port:-80}"
		path="${path:-/}"
		
		# Create HTTP request
		local request=""
		request+="$method $path HTTP/1.1\r\n"
		request+="Host: $host\r\n"
		request+="User-Agent: bash-test-client/1.0\r\n"
		request+="Connection: close\r\n"
		request+="\r\n"
		
		# Send request and capture response
		local response=""
		if [ "$port" = "443" ]; then
			# For HTTPS, we'll use openssl s_client as fallback since pure bash can't handle TLS
			response=$(echo -e "$request" | openssl s_client -connect "$host:$port" -quiet 2>/dev/null)
		else
			# For HTTP, use pure bash
			response=$(echo -e "$request" | bash -c "exec 3<>/dev/tcp/$host/$port; cat >&3; cat <&3; exec 3<&-" 2>/dev/null)
		fi
		
		echo "$response"
	}
	
	# Test HTTP response parsing
	parse_http_response() {
		local response="$1"
		local expected_content="$2"
		
		# Check if response contains HTTP status line
		if echo "$response" | grep -q "HTTP/1.1 200 OK\|HTTP/1.0 200 OK"; then
			# Check if response contains expected content
			if [ -n "$expected_content" ]; then
				echo "$response" | grep -q "$expected_content"
			else
				return 0
			fi
		else
			return 1
		fi
	}
	
	echo "DEBUG: Starting HTTP test with 10 second timeout..."
	echo === HTTP TEST ===
	HTTP_RESPONSE=$(http_request "GET" "http://$SRV" "$SRV" "80" "/" "10")
	if parse_http_response "$HTTP_RESPONSE"; then
		echo "DEBUG: HTTP test completed successfully"
	else
		echo "ERROR: HTTP test failed"
		echo "Response: '$HTTP_RESPONSE'"
		exit 1
	fi
	
	# Test serving additional files from /var/www/html
	echo "DEBUG: Testing file serving from /var/www/html..."
	echo === FILE SERVING TEST ===
	
	# Test serving test.txt
	TEST_TXT_RESPONSE=$(http_request "GET" "http://$SRV/test.txt" "$SRV" "80" "/test.txt" "10")
	if parse_http_response "$TEST_TXT_RESPONSE" "Test file content"; then
		echo "DEBUG: test.txt served correctly from /var/www/html"
	else
		echo "ERROR: test.txt not served correctly from /var/www/html"
		echo "Response: '$TEST_TXT_RESPONSE'"
		exit 1
	fi
	
	# Test serving test.html
	TEST_HTML_RESPONSE=$(http_request "GET" "http://$SRV/test.html" "$SRV" "80" "/test.html" "10")
	if parse_http_response "$TEST_HTML_RESPONSE" "Test Page"; then
		echo "DEBUG: test.html served correctly from /var/www/html"
	else
		echo "ERROR: test.html not served correctly from /var/www/html"
		echo "Response: '$TEST_HTML_RESPONSE'"
		exit 1
	fi
	
	# Test serving the original index.html
	INDEX_RESPONSE=$(http_request "GET" "http://$SRV/" "$SRV" "80" "/" "10")
	if parse_http_response "$INDEX_RESPONSE"; then
		echo "DEBUG: Original index.html served correctly from /var/www/html"
	else
		echo "ERROR: Original index.html not served correctly from /var/www/html"
		echo "Response: '$INDEX_RESPONSE'"
		exit 1
	fi
	
	# For HTTPS tests, we'll use openssl s_client since pure bash can't handle TLS
	echo "DEBUG: Testing HTTPS file serving from /var/www/html..."
	echo === HTTPS TEST ===
	
	# Test HTTPS serving of files using openssl s_client
	HTTPS_TXT_RESPONSE=$(echo -e "GET /test.txt HTTP/1.1\r\nHost: $SRV\r\nConnection: close\r\n\r\n" | openssl s_client -connect "$SRV:443" -quiet 2>/dev/null)
	if echo "$HTTPS_TXT_RESPONSE" | grep -q "HTTP/1.1 200 OK\|HTTP/1.0 200 OK" && echo "$HTTPS_TXT_RESPONSE" | grep -q "Test file content"; then
		echo "DEBUG: HTTPS test.txt served correctly from /var/www/html"
	else
		echo "ERROR: HTTPS test.txt not served correctly from /var/www/html"
		echo "Response: '$HTTPS_TXT_RESPONSE'"
		exit 1
	fi
	
	HTTPS_HTML_RESPONSE=$(echo -e "GET /test.html HTTP/1.1\r\nHost: $SRV\r\nConnection: close\r\n\r\n" | openssl s_client -connect "$SRV:443" -quiet 2>/dev/null)
	if echo "$HTTPS_HTML_RESPONSE" | grep -q "HTTP/1.1 200 OK\|HTTP/1.0 200 OK" && echo "$HTTPS_HTML_RESPONSE" | grep -q "Test Page"; then
		echo "DEBUG: HTTPS test.html served correctly from /var/www/html"
	else
		echo "ERROR: HTTPS test.html not served correctly from /var/www/html"
		echo "Response: '$HTTPS_HTML_RESPONSE'"
		exit 1
	fi
	
	echo "DEBUG: All file serving tests passed - server is correctly serving from /var/www/htmls "
	
	sleep 1
	
	echo "DEBUG: Testing certificate stability..."
	echo === CERTIFICATE STABILITY TEST ===
	CERT1=$(openssl s_client -connect $SRV:443 -servername $SRV < /dev/null 2>/dev/null | openssl x509 -fingerprint -sha256 -noout 2>/dev/null | cut -d= -f2)
	sleep 3
	CERT2=$(openssl s_client -connect $SRV:443 -servername $SRV < /dev/null 2>/dev/null | openssl x509 -fingerprint -sha256 -noout 2>/dev/null | cut -d= -f2)
	
	if [ -z "$CERT1" ] || [ -z "$CERT2" ]; then
		echo "ERROR: Certificate fingerprint test failed - could not retrieve certificates"
		exit 1
	fi
	
	if [ "$CERT1" != "$CERT2" ]; then
		echo "ERROR: Certificate stability test failed - certificate changed between requests"
		echo "   First cert:  $CERT1"
		echo "   Second cert: $CERT2"
		exit 1
	else
		echo "DEBUG: Certificate stability test passed - same certificate on multiple requests"
	fi
	
	# Simple file download test using bash
	echo "DEBUG: Testing file download with bash..."
	echo === DOWNLOAD TEST ===
	
	# Create a temporary directory for downloads
	DOWNLOAD_DIR="/tmp/bash_test_$$"
	mkdir -p "$DOWNLOAD_DIR"
	
	# Download test.txt using bash
	echo -e "GET /test.txt HTTP/1.1\r\nHost: $SRV\r\nConnection: close\r\n\r\n" | bash -c "exec 3<>/dev/tcp/$SRV/80; cat >&3; cat <&3; exec 3<&-" > "$DOWNLOAD_DIR/test.txt"
	
	# Check if file was downloaded and contains expected content
	if [ -f "$DOWNLOAD_DIR/test.txt" ] && grep -q "Test file content" "$DOWNLOAD_DIR/test.txt"; then
		echo "DEBUG: File download test passed"
	else
		echo "ERROR: File download test failed"
		rm -rf "$DOWNLOAD_DIR"
		exit 1
	fi
	
	# Clean up
	rm -rf "$DOWNLOAD_DIR"
	
	echo === END TESTS ===
	
	if [ -z "$KEEPALIVE" ]
	then
		pkill easyp 2>/dev/null || true
		sleep 1
		pkill -9 easyp 2>/dev/null || true
		echo "DEBUG: Stopping server process on local machine..."
		
		# Clean up only the test files we created (leave original content intact)
		echo "DEBUG: Cleaning up test files"
		rm -f /var/www/html/test.txt
		rm -f /var/www/html/test.html
		rm -f /var/www/html/css/test.css
		rm -f /var/www/html/js/test.js
		rmdir /var/www/html/css /var/www/html/js 2>/dev/null || true
	fi
	
	echo "DEBUG: Local test script completed successfully"
	kill $easypid
	exit 0
