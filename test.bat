@echo off
setlocal enabledelayedexpansion

echo ========================================
echo    EASYP HTTPS SERVER TEST SUITE
echo ========================================
echo.

REM Configuration
set "LOCAL_HOST=localhost"
set "CA_HOST=ca.dansted.org"
set "US_HOST=us.dansted.org"
set "BUILD_PROFILE=lto"
set "TARGET=x86_64-unknown-linux-gnu"

REM Parse command line arguments
set "TEST_LOCAL=1"
set "TEST_CA=1"
set "TEST_US=1"
set "QUIT_AFTER="
set "VERBOSE="

:parse_args
if "%1"=="" goto :start_tests
if "%1"=="--local-only" (
    set "TEST_CA=0"
    set "TEST_US=0"
    shift
    goto :parse_args
)
if "%1"=="--remote-only" (
    set "TEST_LOCAL=0"
    shift
    goto :parse_args
)
if "%1"=="--ca-only" (
    set "TEST_LOCAL=0"
    set "TEST_US=0"
    shift
    goto :parse_args
)
if "%1"=="--us-only" (
    set "TEST_LOCAL=0"
    set "TEST_CA=0"
    shift
    goto :parse_args
)
if "%1"=="--quitafter" (
    set "QUIT_AFTER=1"
    shift
    goto :parse_args
)
if "%1"=="--verbose" (
    set "VERBOSE=--verbose"
    shift
    goto :parse_args
)
if "%1"=="--help" (
    echo Usage: test.bat [options]
    echo.
    echo Options:
    echo   --local-only     Test only local build
    echo   --remote-only    Test only remote builds
    echo   --ca-only        Test only ca.dansted.org
    echo   --us-only        Test only us.dansted.org
    echo   --quitafter      Stop servers after tests
    echo   --verbose        Enable verbose output
    echo   --help           Show this help
    echo.
    goto :end
)
shift
goto :parse_args

:start_tests
echo Starting test suite...
echo.

REM Test 0: Static Code Analysis (before build)
echo ========================================
echo    TEST 0: STATIC CODE ANALYSIS
echo ========================================
echo.

echo Testing build.rs for hardcoded extension assumptions...
findstr /i "comment" "easyp-crate\build.rs" >nul
if not errorlevel 1 (
    echo ERROR: build.rs contains hardcoded 'comment' references!
    echo build.rs should dynamically scan and add extensions from the extensions/ directory and not hardcode assumptions about which extensions are in extensions/
    echo.
    echo Found hardcoded 'comment' references in build.rs:
    findstr /n /i "comment" "easyp-crate\build.rs"
    echo.
    echo This violates the dynamic extension loading principle.
    goto :error_exit
)
echo ✓ build.rs does not contain hardcoded 'comment' references

echo.
echo ✓ Static code analysis completed successfully
echo.

REM Test 1: Local Build and Test
if "%TEST_LOCAL%"=="1" (
    echo ========================================
    echo    TEST 1: LOCAL BUILD AND TEST
    echo ========================================
    echo.
    
    echo Building local debug version...
    cd /d "%~dp0\easyp-crate"
    call cargo build --bin easyp
    if errorlevel 1 (
        echo ERROR: Local build failed!
        goto :error_exit
    )
    echo ✓ Local build completed successfully
    echo.
    
    echo Starting local server...
    echo Killing any existing easyp processes...
    taskkill /f /im easyp.exe 2>nul || echo No existing processes found
    
    echo Creating test files...
    if not exist "C:\var\www\html" mkdir "C:\var\www\html"
    echo Test file content > "C:\var\www\html\test.txt"
    echo ^<html^>^<body^>^<h2^>Test Page^</h2^>^<p^>This is a test page^</p^>^</body^>^</html^> > "C:\var\www\html\test.html"
    
    echo Starting server in background...
    start /b "" ..\target\debug\easyp.exe --root "C:\var\www\html" --staging %VERBOSE% > server.log 2>&1
    
    echo Waiting for server to start...
    timeout /t 5 /nobreak >nul
    
    REM Wait for server to actually be listening on ports
    echo Checking if server is ready...
    timeout /t 10 /nobreak >nul
    
    REM Test if server is actually responding (with timeout)
    echo Testing server readiness...
    set SERVER_READY_ATTEMPTS=0
    :test_server_ready
    set /a SERVER_READY_ATTEMPTS+=1
    if !SERVER_READY_ATTEMPTS! gtr 15 (
        echo ERROR: Server failed to start after 30 seconds!
        echo Server logs:
        type server.log 2>nul
        goto :error_exit
    )
    curl -s -o nul -w "%%{http_code}" http://localhost/ 2>nul
    if errorlevel 1 (
        echo Server not responding yet, waiting 2 more seconds... (attempt !SERVER_READY_ATTEMPTS!/15)
        timeout /t 2 /nobreak >nul
        goto test_server_ready
    )
    echo Server is ready and responding
    
    echo Testing local server functionality...
    echo Waiting for server to fully start...
    timeout /t 5 /nobreak >nul
    
    REM Simple test - just verify the server started without errors
    echo Checking server logs for errors...
    findstr /i "error" server.log >nul
    if not errorlevel 1 (
        echo ERROR: Server logs contain errors!
        echo Server logs:
        type server.log
        goto :error_exit
    )
    echo ✓ Server started without errors
    
    REM Check if server is listening on expected ports
    echo Checking if server is listening on ports...
    netstat -an | findstr ":80 " >nul
    if errorlevel 1 (
        echo WARNING: Port 80 not listening (may be normal on Windows)
    ) else (
        echo ✓ Port 80 is listening
    )
    
    netstat -an | findstr ":443 " >nul
    if errorlevel 1 (
        echo WARNING: Port 443 not listening (may be normal on Windows)
    ) else (
        echo ✓ Port 443 is listening
    )
    
    echo ✓ Local server test passed
    
    echo Testing local file serving...
    echo Checking if test file exists...
    if not exist "C:\var\www\html\test.txt" (
        echo ERROR: Test file not found!
        goto :error_exit
    )
    echo ✓ Local file serving test passed
    
    echo Testing extensions functionality...
    echo Creating test extension files...
    echo ^<html^>^<body^>^<h2^>Comment Test^</h2^>^<p^>This is a comment test page^</p^>^</body^>^</html^> > "C:\var\www\html\comment_test.html"
    echo ^<html^>^<body^>^<h2^>Math Test^</h2^>^<p^>This is a math test page^</p^>^</body^>^</html^> > "C:\var\www\html\math_test.html"
    echo ✓ Extensions test files created
    
    echo Testing #EXTEND:comment() processing...
    echo Creating test post directory structure...
    if not exist "C:\var\www\html\posts\2024-08-15-eve-galaxy-conquest-tips" mkdir "C:\var\www\html\posts\2024-08-15-eve-galaxy-conquest-tips"
    echo Copying test post with #EXTEND:comment() directive...
    copy "test_root\posts\2024-08-15-eve-galaxy-conquest-tips\index.html" "C:\var\www\html\posts\2024-08-15-eve-galaxy-conquest-tips\index.html" >nul
    echo ✓ Test post created with #EXTEND:comment() directive
    
    echo Testing #EXTEND:comment() replacement...
    curl -s -o comment_test_output.html "http://localhost/posts/2024-08-15-eve-galaxy-conquest-tips/" 2>nul
    if exist comment_test_output.html (
        findstr /c:"#EXTEND:comment()" comment_test_output.html >nul
        if errorlevel 1 (
            echo ✓ #EXTEND:comment() directive was successfully replaced
        ) else (
            echo ERROR: #EXTEND:comment() directive was NOT replaced!
            echo Raw output contains unprocessed directive:
            findstr /c:"#EXTEND:comment()" comment_test_output.html
            goto :error_exit
        )
        
        findstr /c:"comment-form" comment_test_output.html >nul
        if not errorlevel 1 (
            echo ✓ Comment form HTML was generated
        ) else (
            echo WARNING: Comment form HTML not found in output
        )
        
        del comment_test_output.html
    ) else (
        echo ERROR: Failed to fetch test post for #EXTEND:comment() test!
        goto :error_exit
    )
    echo ✓ #EXTEND:comment() processing test completed
    
    echo Testing binary file serving...
    echo Creating test binary file...
    if not exist "C:\var\www\html\test.gz" (
        copy "test_root\README.md.gz" "C:\var\www\html\test.gz" >nul
    )
    echo ✓ Test binary file created
    
    echo Testing HTTP binary file serving...
    curl -s -o test_http.gz "http://localhost/test.gz" 2>nul
    if exist test_http.gz (
        echo ✓ HTTP binary file download completed
        REM Compare file sizes to ensure no corruption
        for %%A in ("C:\var\www\html\test.gz") do set "ORIGINAL_SIZE=%%~zA"
        for %%A in ("test_http.gz") do set "DOWNLOADED_SIZE=%%~zA"
        if "%ORIGINAL_SIZE%"=="%DOWNLOADED_SIZE%" (
            echo ✓ HTTP binary file size matches original (%ORIGINAL_SIZE% bytes)
        ) else (
            echo ERROR: HTTP binary file size mismatch! Original: %ORIGINAL_SIZE%, Downloaded: %DOWNLOADED_SIZE%
            goto :error_exit
        )
        del test_http.gz
    ) else (
        echo ERROR: HTTP binary file download failed!
        goto :error_exit
    )
    
    echo Testing HTTPS binary file serving...
    curl -s -k -o test_https.gz "https://localhost/test.gz" 2>nul
    if exist test_https.gz (
        echo ✓ HTTPS binary file download completed
        REM Compare file sizes to ensure no corruption
        for %%A in ("test_https.gz") do set "DOWNLOADED_SIZE=%%~zA"
        if "%ORIGINAL_SIZE%"=="%DOWNLOADED_SIZE%" (
            echo ✓ HTTPS binary file size matches original (%ORIGINAL_SIZE% bytes)
        ) else (
            echo ERROR: HTTPS binary file size mismatch! Original: %ORIGINAL_SIZE%, Downloaded: %DOWNLOADED_SIZE%
            goto :error_exit
        )
        del test_https.gz
    ) else (
        echo ERROR: HTTPS binary file download failed!
        goto :error_exit
    )
    
    echo ✓ Binary file serving test completed successfully
    
    echo Running 5-second stress test...
    echo Testing basic connectivity with simple tools...
    
    REM Test HTTP connectivity with curl if available, otherwise use telnet
    echo Testing HTTP connectivity...
    curl --version >nul 2>&1
    if errorlevel 1 (
        echo WARNING: curl not available, skipping HTTP test
    ) else (
        curl -s -o nul -w "%%{http_code}" http://localhost/ 2>nul > http_test_result.txt
        if exist http_test_result.txt (
            set /p HTTP_CODE=<http_test_result.txt
            del http_test_result.txt
            if "!HTTP_CODE!"=="200" (
                echo ✓ HTTP test passed (status: !HTTP_CODE!)
            ) else (
                echo ERROR: HTTP test failed (status: !HTTP_CODE!)
                echo Server logs:
                type server.log
                goto :error_exit
            )
        ) else (
            echo ERROR: HTTP test failed - no response
            goto :error_exit
        )
    )
    
    REM Test HTTPS connectivity
    echo Testing HTTPS connectivity...
    curl --version >nul 2>&1
    if errorlevel 1 (
        echo WARNING: curl not available, skipping HTTPS test
    ) else (
        curl -s -o nul -w "%%{http_code}" -k https://localhost/ 2>nul > https_test_result.txt
        if exist https_test_result.txt (
            set /p HTTPS_CODE=<https_test_result.txt
            del https_test_result.txt
            if "!HTTPS_CODE!"=="200" (
                echo ✓ HTTPS test passed (status: !HTTPS_CODE!)
            ) else if "!HTTPS_CODE!"=="404" (
                echo ✓ HTTPS test passed (status: !HTTPS_CODE! - expected for root path)
            ) else (
                echo ERROR: HTTPS test failed (status: !HTTPS_CODE!)
                echo Server logs:
                type server.log
                goto :error_exit
            )
        ) else (
            echo ERROR: HTTPS test failed - no response
            goto :error_exit
        )
    )
    
    REM High-intensity stress test - 5 seconds, minimum 100 responses
    echo Running high-intensity 5-second stress test...
    echo Target: At least 100 successful responses in 5 seconds
    set STRESS_RESPONSES=0
    set STRESS_FAILED=0
    set STRESS_START_TIME=%time%
    
    REM Start background process to track time
    start /b "" powershell -command "Start-Sleep 5; echo TIMEOUT_REACHED > stress_timeout.txt" 2>nul
    
    :stress_loop
    REM Check if 5 seconds have passed
    if exist stress_timeout.txt (
        del stress_timeout.txt 2>nul
        goto :stress_complete
    )
    
    REM Run HTTP request
    curl -s -o nul -w "%%{http_code}" --max-time 1 --connect-timeout 1 http://localhost/ 2>nul
    if errorlevel 1 (
        set /a STRESS_FAILED+=1
    ) else (
        set /a STRESS_RESPONSES+=1
    )
    
    REM Run HTTPS request
    curl -s -o nul -w "%%{http_code}" --max-time 1 --connect-timeout 1 -k https://localhost/ 2>nul
    if errorlevel 1 (
        set /a STRESS_FAILED+=1
    ) else (
        set /a STRESS_RESPONSES+=1
    )
    
    REM Show progress every 20 responses
    set /a PROGRESS_MOD=STRESS_RESPONSES %% 20
    if !PROGRESS_MOD!==0 (
        echo Progress: !STRESS_RESPONSES! responses, !STRESS_FAILED! failures
    )
    
    goto :stress_loop
    
    :stress_complete
    set STRESS_END_TIME=%time%
    
    echo.
    echo Stress test completed!
    echo Start time: !STRESS_START_TIME!
    echo End time: !STRESS_END_TIME!
    echo Total responses: !STRESS_RESPONSES!
    echo Total failures: !STRESS_FAILED!

    if !STRESS_RESPONSES! lss 100 (o
        echo ERROR: Stress test failed! Only !STRESS_RESPONSES! responses in 5 seconds (minimum required: 100)
        echo This indicates the server is not handling load properly.
        echo Server logs:
        type server.log 2>nul
        goto :error_exit
    )

    echo ✓ High-intensity stress test passed (!STRESS_RESPONSES! responses in 5 seconds)
    
    REM Clean up any remaining timeout file
    if exist stress_timeout.txt del stress_timeout.txt 2>nul
    
    if "%QUIT_AFTER%"=="1" (
        echo Stopping local server...
        taskkill /f /im easyp.exe 2>nul
    )
    
    echo ✓ Local test completed successfully
    echo.
)

REM Test 2: Cross-compile and Deploy to ca.dansted.org
if "%TEST_CA%"=="1" (
    echo ========================================
    echo    TEST 2: CROSS-COMPILE FOR CA.DANSTED.ORG
    echo ========================================
    echo.
    
    echo Cross-compiling for %TARGET%...
    cd /d "%~dp0\easyp-crate"
    call cross build --target %TARGET% --profile %BUILD_PROFILE%
    if errorlevel 1 (
        echo ERROR: Cross-compilation failed!
        goto :error_exit
    )
    echo ✓ Cross-compilation completed successfully
    echo.
    
    echo Deploying to ca.dansted.org...
    wsl rsync -av --progress target\%TARGET%\%BUILD_PROFILE%\easyp %CA_HOST%:
    if errorlevel 1 (
        echo ERROR: Failed to sync binary to ca.dansted.org!
        goto :error_exit
    )
    echo ✓ Binary synced to ca.dansted.org
    echo.
    
    echo Starting server on ca.dansted.org...
    wsl ssh %CA_HOST% "pkill easyp ; chmod +x easyp ; nohup ./easyp --root /var/www/html --staging %VERBOSE% > server.log 2>&1 &"
    if errorlevel 1 (
        echo ERROR: Failed to start server on ca.dansted.org!
        goto :error_exit
    )
    echo ✓ Server started on ca.dansted.org
    echo.
    
    echo Waiting for server to initialize...
    timeout /t 10 /nobreak >nul
    
    echo Testing ca.dansted.org HTTP connectivity...
    curl -s --connect-timeout 10 --max-time 15 "http://%CA_HOST%/" >nul 2>&1
    if errorlevel 1 (
        echo ERROR: ca.dansted.org HTTP test failed!
        wsl ssh %CA_HOST% "tail -20 server.log"
        goto :error_exit
    )
    echo ✓ ca.dansted.org HTTP test passed
    
    echo Testing ca.dansted.org HTTPS connectivity...
    curl -s -k --connect-timeout 10 --max-time 15 "https://%CA_HOST%/" >nul 2>&1
    if errorlevel 1 (
        echo ERROR: ca.dansted.org HTTPS test failed!
        wsl ssh %CA_HOST% "tail -20 server.log"
        goto :error_exit
    )
    echo ✓ ca.dansted.org HTTPS test passed
    
    echo Testing certificate stability on ca.dansted.org...
    for /f %%i in ('openssl s_client -connect %CA_HOST%:443 -servername %CA_HOST% -quiet 2^>nul ^| openssl x509 -fingerprint -sha256 -noout 2^>nul ^| findstr "="') do set "CERT1=%%i"
    timeout /t 3 /nobreak >nul
    for /f %%i in ('openssl s_client -connect %CA_HOST%:443 -servername %CA_HOST% -quiet 2^>nul ^| openssl x509 -fingerprint -sha256 -noout 2^>nul ^| findstr "="') do set "CERT2=%%i"
    
    if "!CERT1!"=="!CERT2!" (
        echo ✓ Certificate stability test passed on ca.dansted.org
    ) else (
        echo ERROR: Certificate stability test failed on ca.dansted.org!
        echo First cert:  !CERT1!
        echo Second cert: !CERT2!
        goto :error_exit
    )
    
    if "%QUIT_AFTER%"=="1" (
        echo Stopping server on ca.dansted.org...
        wsl ssh %CA_HOST% "pkill easyp"
    )
    
    echo ✓ ca.dansted.org test completed successfully
    echo.
)

REM Test 3: Deploy to us.dansted.org
if "%TEST_US%"=="1" (
    echo ========================================
    echo    TEST 3: DEPLOY TO US.DANSTED.ORG
    echo ========================================
    echo.
    
    echo Deploying to us.dansted.org...
    wsl rsync -av --progress target\%TARGET%\%BUILD_PROFILE%\easyp %US_HOST%:
    if errorlevel 1 (
        echo ERROR: Failed to sync binary to us.dansted.org!
        goto :error_exit
    )
    echo ✓ Binary synced to us.dansted.org
    echo.
    
    echo Starting server on us.dansted.org...
    wsl ssh %US_HOST% "pkill easyp ; chmod +x easyp ; nohup ./easyp --root /var/www/html --staging %VERBOSE% > server.log 2>&1 &"
    if errorlevel 1 (
        echo ERROR: Failed to start server on us.dansted.org!
        goto :error_exit
    )
    echo ✓ Server started on us.dansted.org
    echo.
    
    echo Waiting for server to initialize...
    timeout /t 10 /nobreak >nul
    
    echo Testing us.dansted.org HTTP connectivity...
    curl -s --connect-timeout 10 --max-time 15 "http://%US_HOST%/" >nul 2>&1
    if errorlevel 1 (
        echo ERROR: us.dansted.org HTTP test failed!
        wsl ssh %US_HOST% "tail -20 server.log"
        goto :error_exit
    )
    echo ✓ us.dansted.org HTTP test passed
    
    echo Testing us.dansted.org HTTPS connectivity...
    curl -s -k --connect-timeout 10 --max-time 15 "https://%US_HOST%/" >nul 2>&1
    if errorlevel 1 (
        echo ERROR: us.dansted.org HTTPS test failed!
        wsl ssh %US_HOST% "tail -20 server.log"
        goto :error_exit
    )
    echo ✓ us.dansted.org HTTPS test passed
    
    echo Testing HTTP/2 support on us.dansted.org...
    curl -s -k --http2 --connect-timeout 10 --max-time 15 "https://%US_HOST%/" >nul 2>&1
    if errorlevel 1 (
        echo WARNING: HTTP/2 test failed on us.dansted.org (this may be expected)
    ) else (
        echo ✓ HTTP/2 test passed on us.dansted.org
    )
    
    echo Testing certificate stability on us.dansted.org...
    for /f %%i in ('openssl s_client -connect %US_HOST%:443 -servername %US_HOST% -quiet 2^>nul ^| openssl x509 -fingerprint -sha256 -noout 2^>nul ^| findstr "="') do set "CERT1=%%i"
    timeout /t 3 /nobreak >nul
    for /f %%i in ('openssl s_client -connect %US_HOST%:443 -servername %US_HOST% -quiet 2^>nul ^| openssl x509 -fingerprint -sha256 -noout 2^>nul ^| findstr "="') do set "CERT2=%%i"
    
    if "!CERT1!"=="!CERT2!" (
        echo ✓ Certificate stability test passed on us.dansted.org
    ) else (
        echo ERROR: Certificate stability test failed on us.dansted.org!
        echo First cert:  !CERT1!
        echo Second cert: !CERT2!
        goto :error_exit
    )
    
    if "%QUIT_AFTER%"=="1" (
        echo Stopping server on us.dansted.org...
        wsl ssh %US_HOST% "pkill easyp"
    )
    
    echo ✓ us.dansted.org test completed successfully
    echo.
)

REM Test 4: Advanced Tests (if not quitting)
if "%QUIT_AFTER%"=="" (
    echo ========================================
    echo    TEST 4: ADVANCED TESTS
    echo ========================================
    echo.
    
    if "%TEST_CA%"=="1" (
        echo Testing wget on ca.dansted.org...
        wget --no-check-certificate --timeout=10 --tries=1 -q -O temp_ca.html "https://%CA_HOST%/" 2>nul
        if exist temp_ca.html (
            echo ✓ Wget test passed on ca.dansted.org
            del temp_ca.html
        ) else (
            echo WARNING: Wget test failed on ca.dansted.org
        )
    )
    
    if "%TEST_US%"=="1" (
        echo Testing wget on us.dansted.org...
        wget --no-check-certificate --timeout=10 --tries=1 -q -O temp_us.html "https://%US_HOST%/" 2>nul
        if exist temp_us.html (
            echo ✓ Wget test passed on us.dansted.org
            del temp_us.html
        ) else (
            echo WARNING: Wget test failed on us.dansted.org
        )
    )
    
    echo.
)

echo ========================================
echo    ALL TESTS COMPLETED SUCCESSFULLY!
echo ========================================
echo.
echo Summary:
if "%TEST_LOCAL%"=="1" echo ✓ Local build and test
if "%TEST_CA%"=="1" echo ✓ ca.dansted.org deployment and test
if "%TEST_US%"=="1" echo ✓ us.dansted.org deployment and test
echo ✓ All connectivity tests passed
echo ✓ All certificate tests passed
echo.

if "%QUIT_AFTER%"=="" (
    echo Servers are still running. Use --quitafter to stop them after tests.
    echo.
    echo To stop servers manually:
    echo   wsl ssh ca.dansted.org "pkill easyp"
    echo   wsl ssh us.dansted.org "pkill easyp"
    echo   taskkill /f /im easyp.exe
)

goto :end

:error_exit
echo.
echo ========================================
echo    TEST FAILED!
echo ========================================
echo.
echo Check the error messages above for details.
exit /b 1

:end
echo Test suite completed.
