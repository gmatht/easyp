@echo off
echo Testing easyp server with tokio-rustls migration...

echo Building server...
cd /d "%~dp0\easyp-crate"
call cargo build --bin easyp
if errorlevel 1 (
    echo ERROR: Build failed!
    exit /b 1
)
echo ✓ Build completed successfully

echo Starting server...
cd /d "%~dp0"
taskkill /f /im easyp.exe 2>nul || echo No existing processes found

echo Creating test files...
if not exist "C:\var\www\html" mkdir "C:\var\www\html"
echo Test file content > "C:\var\www\html\test.txt"
copy "test_root\README.md.gz" "C:\var\www\html\test.gz" >nul

echo Starting server in background...
start /b "" target\debug\easyp.exe --root "C:\var\www\html" --staging > server.log 2>&1

echo Waiting for server to start...
timeout /t 5 /nobreak >nul

echo Testing HTTP binary file serving...
curl.exe --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: curl not available!
    goto :error_exit
)
curl.exe -s "http://localhost/test.gz" > test_http.gz 2>nul
if exist test_http.gz (
    echo ✓ HTTP binary file download completed
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
curl.exe -s -k "https://localhost/test.gz" > test_https.gz 2>nul
if exist test_https.gz (
    echo ✓ HTTPS binary file download completed
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

echo ✓ All tests passed! tokio-rustls migration successful.
taskkill /f /im easyp.exe 2>nul
goto :end

:error_exit
echo ERROR: Test failed!
taskkill /f /im easyp.exe 2>nul
exit /b 1

:end
echo Test completed.
