@echo off
REM One-command dev loop for Windows:
REM   1. warns if the local node isn't listening,
REM   2. runs the watcher (initial publish + auto-republish on
REM      shared/contract/delegate edits) in a separate window,
REM   3. runs `trunk serve` in this window for the frontend.
REM
REM Loop semantics mirror the bash version:
REM   - editing frontend\src\**       -> trunk rebuilds + hot-reloads
REM   - editing shared\, *-contract\, identity-delegate\ -> watcher
REM     republishes, rewrites dev-keys.json, trunk picks it up.
REM
REM Closing this window (or Ctrl-C on trunk) terminates the watcher
REM window too via taskkill on its WINDOWTITLE.

setlocal EnableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
pushd "%SCRIPT_DIR%.." >nul
set "HERE=%CD%"
popd >nul

if "%WS_PORT%"=="" set "WS_PORT=7509"

netstat -ano | findstr "LISTENING" | findstr ":%WS_PORT% " >nul
if errorlevel 1 (
    echo [dev] WARNING: nothing listening on 127.0.0.1:%WS_PORT%
    echo [dev] start a local node first -- use the LOCAL-BUILT binary,
    echo [dev] NOT `freenet` from PATH: dev-publish.bat runs fdev from
    echo [dev] freenet-core\target\debug\, and a node from a different
    echo [dev] version fails publish with
    echo [dev]   "unknown import: freenet_contract_io::__frnt__fill_buffer".
    echo [dev] example:
    echo       %HERE%\..\freenet-core\target\debug\freenet.exe local --ws-api-address 0.0.0.0 --ws-api-port %WS_PORT% --data-dir %TEMP%\freenet-local
    echo [dev] continuing anyway -- publish will fail loudly if the node isn't up.
)

set "WATCH_TITLE=freenet-idle-dev-watch"
start "%WATCH_TITLE%" cmd /c call "%HERE%\scripts\dev-watch.bat"

echo [dev] watcher started in separate window titled "%WATCH_TITLE%"
echo [dev] starting trunk serve (Ctrl-C to stop)
cd /d "%HERE%\frontend"
trunk serve

REM trunk exited -- tear down the watcher window and any cargo/fdev
REM children it spawned.
taskkill /FI "WINDOWTITLE eq %WATCH_TITLE%" /T /F >nul 2>&1

endlocal
