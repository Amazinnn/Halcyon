@echo off
setlocal
cd /d "%~dp0apps\desktop"

set "EXE=%~dp0apps\desktop\src-tauri\target\release\desktop.exe"

set "REBUILD="
set "MONITOR="
if /I "%~1"=="rebuild" set REBUILD=1
if /I "%~1"=="monitor" set MONITOR=1
if /I "%~2"=="monitor" set MONITOR=1

if defined REBUILD goto build
if exist "%EXE%" goto launch
goto build

:build
where node >nul 2>nul
if errorlevel 1 (
  echo [Focus] Node.js not found. Please install Node.js first.
  pause
  exit /b 1
)
echo [Focus] Building release (this takes a few minutes)...
call npm run tauri build -- --no-bundle
if errorlevel 1 (
  echo [Focus] Build failed.
  pause
  exit /b 1
)

:launch
if not exist "%EXE%" (
  echo [Focus] Release binary not found: %EXE%
  pause
  exit /b 1
)
echo [Focus] Starting Focus Desktop...
start "" "%EXE%" <nul >nul 2>nul

if defined MONITOR (
  echo [Focus] Starting hang monitor (scripts\hang-detector.ps1, Ctrl+C in that window to stop)...
  start "Focus Hang Monitor" powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\hang-detector.ps1" -ProcessName desktop -Seconds 0 -LogFile "%APPDATA%\com.focusdesktop.app\hang-detector.log" -DumpDir "%APPDATA%\com.focusdesktop.app\hangs"
)
exit /b 0