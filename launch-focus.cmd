@echo off
setlocal
cd /d "%~dp0apps\desktop"

set "EXE=%~dp0apps\desktop\src-tauri\target\release\desktop.exe"

if /I "%~1"=="rebuild" goto build
if exist "%EXE%" goto launch

:build
where node >nul 2>nul
if errorlevel 1 (
  echo [Focus] Node.js not found. Please install Node.js first.
  pause
  exit /b 1
)
echo [Focus] First launch: building release (this takes a few minutes)...
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
exit /b 0
