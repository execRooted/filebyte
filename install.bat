@echo off
setlocal enabledelayedexpansion

echo filebyte installer
echo ==================
echo by execRooted
echo.

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Rust/Cargo is not installed or not in PATH.
    echo [INFO] Please install Rust from https://www.rust-lang.org/tools/install
    pause
    exit /b 1
)

echo [INFO] Building filebyte...
cd /d "%~dp0"
if not exist "Cargo.toml" (
    echo [ERROR] Cargo.toml not found. Please run this script from the project root.
    pause
    exit /b 1
)

cargo build --release
if %errorlevel% neq 0 (
    echo [ERROR] Build failed.
    pause
    exit /b 1
)

set "INSTALL_DIR=%USERPROFILE%\.cargo\bin"
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"

echo [INFO] Installing filebyte to %INSTALL_DIR%...
copy /Y "target\release\filebyte.exe" "%INSTALL_DIR%\filebyte.exe" >nul
copy /Y "target\release\fbt.exe" "%INSTALL_DIR%\fbt.exe" >nul

echo.
echo [SUCCESS] Installation complete!
echo [INFO] You can now run 'filebyte' or 'fbt' from anywhere.
echo [USAGE] To run the tool, simply type: filebyte
echo [USAGE] For help run: filebyte -h
echo.
pause
