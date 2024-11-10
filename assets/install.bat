@echo off

set SCRIPT_DIR=%~dp0

REM Check for administrative privileges
net session >nul 2>&1
if %errorlevel% neq 0 (
    REM Elevate script to run as administrator
    echo Requesting administrative privileges...
    powershell -Command "Start-Process '%0' -Verb runAs -WorkingDirectory '%SCRIPT_DIR%'"
    exit /b
)

START /B %~dp0\bg3_autostart.exe --install
