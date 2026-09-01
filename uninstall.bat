@echo off
setlocal enabledelayedexpansion

echo filebyte uninstaller
echo =====================
echo by execRooted
echo.

set "INSTALL_DIR=%USERPROFILE%\.cargo\bin"

if exist "%INSTALL_DIR%\filebyte.exe" (
    echo [INFO] Removing filebyte from %INSTALL_DIR%...
    del /F "%INSTALL_DIR%\filebyte.exe"
    echo [SUCCESS] filebyte removed.
) else (
    echo [INFO] filebyte is not installed in %INSTALL_DIR%.
)

if exist "%INSTALL_DIR%\fbt.exe" (
    echo [INFO] Removing fbt from %INSTALL_DIR%...
    del /F "%INSTALL_DIR%\fbt.exe"
    echo [SUCCESS] fbt removed.
) else (
    echo [INFO] fbt is not installed in %INSTALL_DIR%.
)

echo.
pause
