@echo off
setlocal enabledelayedexpansion

REM PUBLISH_VERSION.bat - Build and upload easyp releases
REM Usage: PUBLISH_VERSION.bat <VERSION> [LATEST]
REM Example: PUBLISH_VERSION.bat 0.1.3 LATEST
REM         PUBLISH_VERSION.bat 0.1.3 (without updating symlinks)

if "%~1"=="" (
    echo Usage: %0 ^<VERSION^> [LATEST]
    echo Example: %0 0.1.3 LATEST
    echo.
    echo This script will:
    echo " 1. Create easyp-VER.tgz source tarball (using WSL tar with maximum compression)"
    echo " 2. Build easyp-VER-x64.gz Linux binary using cross with LTO and gz99 compression"
    echo " 3. Build easyp-VER-x64.zip Windows binary with LTO and maximum compression"
    echo " 4. Upload all files to www.easyp.net:/var/www/html"
    echo " 5. If LATEST is specified, update easyp-latest symlinks"
    echo.
    echo Requirements:
    echo " - WSL (Windows Subsystem for Linux)"
    echo " - cross (cargo install cross)"
    echo " - SSH access to www.easyp.net"
    echo " - PowerShell (for Windows binary compression)"
    exit /b 1
)

set VERSION=%1
set IS_LATEST=%2
set UPLOAD_HOST=www.easyp.net
set UPLOAD_PATH=/var/www/html

echo Building easyp version %VERSION%...
echo.

REM Check if we're in the right directory
if not exist "Cargo.toml" (
    echo Error: Cargo.toml not found. Please run this script from the project root.
    exit /b 1
)

REM Check if cross is installed
where cross >nul 2>&1
if errorlevel 1 (
    echo Error: cross is not installed. Please install it with: cargo install cross
    exit /b 1
)

REM Check if WSL is available
wsl --version >nul 2>&1
if errorlevel 1 (
    echo Error: WSL is not available. Please install WSL or use Windows Subsystem for Linux.
    exit /b 1
)

REM Create temporary directory for builds
set TEMP_DIR=%TEMP%\easyp-build-%RANDOM%
mkdir "%TEMP_DIR%"

echo Step 1: Creating source tarball easyp-%VERSION%.tgz...
REM Create source tarball excluding build artifacts using WSL
REM Convert Windows path to WSL path
for /f "tokens=*" %%i in ('wsl wslpath -a "%TEMP_DIR%"') do set WSL_TEMP_DIR=%%i
if errorlevel 1 (
    echo Error: Failed to convert Windows path to WSL path
    goto cleanup
)
wsl env GZIP=-9 tar -zcf "%WSL_TEMP_DIR%/easyp-%VERSION%.tgz" --exclude="*/target/*" --exclude="target/" --exclude="*.log" --exclude="*.tmp" --exclude="*.bak" --exclude="*/ubuntu-12.04-rootfs/*" .
if errorlevel 1 (
    echo Error: Failed to create source tarball
    goto cleanup
)
echo Source tarball created: %TEMP_DIR%\easyp-%VERSION%.tgz

echo.
echo Step 2: Building Linux binary with LTO...
REM Build Linux binary using cross with LTO profile
cross build --profile lto --target x86_64-unknown-linux-gnu --bin easyp
if errorlevel 1 (
    echo Error: Failed to build Linux binary
    goto cleanup
)

REM Compress the Linux binary using WSL with gz99 for maximum compression
wsl env ./gz99 -c < "target/x86_64-unknown-linux-gnu/lto/easyp" "%WSL_TEMP_DIR%/easyp-%VERSION%-x64.gz"
if errorlevel 1 (
    echo Error: Failed to compress Linux binary
    goto cleanup
)
echo Linux binary created: %TEMP_DIR%\easyp-%VERSION%-x64.gz

echo.
echo Step 3: Building Windows binary with LTO...
REM Build Windows binary with LTO profile
cargo build --profile lto --bin easyp
if errorlevel 1 (
    echo Error: Failed to build Windows binary
    goto cleanup
)

REM Create Windows binary zip with maximum compression
powershell -Command "Compress-Archive -Path 'target\lto\easyp.exe' -DestinationPath '%TEMP_DIR%\easyp-%VERSION%-x64.zip' -CompressionLevel Optimal -Force"
if errorlevel 1 (
    echo Error: Failed to create Windows binary zip
    goto cleanup
)
echo Windows binary created: %TEMP_DIR%\easyp-%VERSION%-x64.zip

echo.
echo Step 4: Verifying files before upload...
REM Verify all files exist
if not exist "%TEMP_DIR%\easyp-%VERSION%.tgz" (
    echo Error: Source tarball not found
    goto cleanup
)
if not exist "%TEMP_DIR%\easyp-%VERSION%-x64.gz" (
    echo Error: Linux binary not found
    goto cleanup
)
if not exist "%TEMP_DIR%\easyp-%VERSION%-x64.zip" (
    echo Error: Windows binary not found
    goto cleanup
)
echo All files verified successfully.

echo.
echo Step 5: Uploading files to %UPLOAD_HOST%:%UPLOAD_PATH%...
REM Upload all files to the server
scp "%TEMP_DIR%\easyp-%VERSION%.tgz" "%UPLOAD_HOST%:%UPLOAD_PATH%/"
if errorlevel 1 (
    echo Error: Failed to upload source tarball
    goto cleanup
)

scp "%TEMP_DIR%\easyp-%VERSION%-x64.gz" "%UPLOAD_HOST%:%UPLOAD_PATH%/"
if errorlevel 1 (
    echo Error: Failed to upload Linux binary
    goto cleanup
)

scp "%TEMP_DIR%\easyp-%VERSION%-x64.zip" "%UPLOAD_HOST%:%UPLOAD_PATH%/"
if errorlevel 1 (
    echo Error: Failed to upload Windows binary
    goto cleanup
)

echo Files uploaded successfully!

REM Update symlinks if LATEST is specified
if /i "%IS_LATEST%"=="LATEST" (
    echo.
    echo Step 6: Updating easyp-latest symlinks...
    ssh %UPLOAD_HOST% "cd %UPLOAD_PATH% && ln -sf easyp-%VERSION%.tgz easyp-latest.tgz && ln -sf easyp-%VERSION%-x64.gz easyp-latest-x64.gz && ln -sf easyp-%VERSION%-x64.zip easyp-latest.zip"
    if errorlevel 1 (
        echo Error: Failed to update symlinks
        goto cleanup
    )
    echo Symlinks updated to point to version %VERSION%
)

echo.
echo ========================================
echo Release %VERSION% published successfully!
echo ========================================
echo.
echo Files uploaded to %UPLOAD_HOST%:%UPLOAD_PATH%:
echo   - easyp-%VERSION%.tgz (source)
echo   - easyp-%VERSION%-x64.gz (Linux binary)
echo   - easyp-%VERSION%-x64.zip (Windows binary)
if /i "%IS_LATEST%"=="LATEST" (
    echo.
    echo Symlinks updated:
    echo   - easyp-latest.tgz
    echo   - easyp-latest-x64.gz
    echo   - easyp-latest.zip
)

goto cleanup

:cleanup
REM Clean up temporary directory
if exist "%TEMP_DIR%" (
    rmdir /s /q "%TEMP_DIR%"
)

endlocal
