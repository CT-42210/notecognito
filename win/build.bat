@echo off
echo Building Notecognito for Windows...
echo.

REM Check if Rust is installed
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: Cargo not found. Please install Rust from https://rustup.rs/
    exit /b 1
)

REM Create assets directory if it doesn't exist
if not exist "assets" (
    echo Creating assets directory...
    mkdir assets
    echo.
    echo IMPORTANT: Add icon.ico to the assets directory before running!
    echo.
)

REM Build in release mode
echo Building release version...
cargo build --release

if %errorlevel% neq 0 (
    echo.
    echo ERROR: Build failed!
    exit /b 1
)

echo.
echo Build successful!
echo.
echo Executable location: target\release\notecognito.exe
echo.

REM Ask if user wants to run
set /p "run=Do you want to run Notecognito now? (y/n): "
if /i "%run%"=="y" (
    echo.
    echo Starting Notecognito...
    start "" "target\release\notecognito.exe"
)

echo.
echo To install permanently:
echo 1. Copy target\release\notecognito.exe to a permanent location
echo 2. Add icon.ico to the same directory
echo 3. Run notecognito.exe
echo.
pause