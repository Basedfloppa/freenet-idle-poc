@echo off
REM Watcher loop (Windows port of scripts/dev-watch.sh):
REM   re-runs scripts\dev-publish.bat whenever any source file
REM   outside frontend\ changes (shared\, *-contract\,
REM   identity-delegate\). Trunk's own watcher handles frontend\src
REM   + style + index.html for hot reload, and picks up the new
REM   dev-keys.json that dev-publish.bat writes on every run.
REM
REM Windows has no inotifywait equivalent in stock cmd, so this
REM always uses a 1-second polling loop driven by PowerShell:
REM hashes (LastWriteTime + path) for every watched file and
REM compares to the previous hash. SHA-256 from .NET is built into
REM PowerShell so there's no extra install.
REM
REM Run alongside `trunk serve`, OR use scripts\dev.bat which starts
REM both in one shot.

setlocal EnableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
pushd "%SCRIPT_DIR%.." >nul
set "HERE=%CD%"
popd >nul

REM Watch lists -- leaf paths only, deliberately no target\ or
REM build\ to avoid the cargo-build-touches-files feedback loop.
set "WATCH_DIRS=%HERE%\shared\src;%HERE%\presence-contract\src;%HERE%\mailbox-contract\src;%HERE%\guilds-contract\src;%HERE%\identity-delegate\src"
set "WATCH_FILES=%HERE%\shared\Cargo.toml;%HERE%\presence-contract\Cargo.toml;%HERE%\mailbox-contract\Cargo.toml;%HERE%\guilds-contract\Cargo.toml;%HERE%\identity-delegate\Cargo.toml"

REM Initial publish so dev-keys.json reflects the current code
REM before any change.
call :republish

echo.
echo [watch] polling every 1 s (inotifywait is not available on Windows)
set "PREV="

:loop
for /f "usebackq delims=" %%S in (`powershell -NoProfile -Command "$dirs=$env:WATCH_DIRS.Split(';'); $files=$env:WATCH_FILES.Split(';'); $items=@(); foreach($d in $dirs){if(Test-Path $d){$items+=Get-ChildItem -Path $d -Recurse -File -Include *.rs,Cargo.toml -ErrorAction SilentlyContinue}}; foreach($f in $files){if(Test-Path $f){$items+=Get-Item $f}}; $s=($items | Sort-Object FullName | ForEach-Object { '{0} {1}' -f $_.LastWriteTime.Ticks,$_.FullName }) -join \"`n\"; [BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes($s))).Replace('-','')"`) do set "STATE=%%S"

if not "!PREV!"=="" (
    if not "!STATE!"=="!PREV!" call :republish
)
set "PREV=!STATE!"

REM Sleep ~1 s. `ping -n 2 127.0.0.1` waits between two pings,
REM i.e. ~1 s; standard cmd idiom since there's no `sleep` builtin.
ping -n 2 127.0.0.1 >nul
goto loop

:republish
echo.
for /f "usebackq delims=" %%t in (`powershell -NoProfile -Command "Get-Date -Format HH:mm:ss"`) do set "NOW=%%t"
echo [watch] !NOW! change detected -- republishing
call "%HERE%\scripts\dev-publish.bat" > "%TEMP%\idle-watch.log" 2>&1
if errorlevel 1 (
    echo [watch] FAILED -- see %TEMP%\idle-watch.log
) else (
    echo [watch] ok. dev-keys.json:
    powershell -NoProfile -Command "$c = Get-Content -Raw '%HERE%\frontend\dev-keys.json'; foreach($m in [regex]::Matches($c, '\"([a-z_]+_b58)\":\s*\"([^\"]+)\"')){ '    {0} = {1}' -f $m.Groups[1].Value,$m.Groups[2].Value }"
)
exit /b 0
