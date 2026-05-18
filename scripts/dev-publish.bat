@echo off
REM Build + publish every artefact the frontend talks to:
REM   1. presence-contract  -> leaderboard / World Boss aggregator
REM   2. mailbox-contract   -> player-to-player signed message log
REM   3. guilds-contract    -> cooperative group registry
REM   4. identity-delegate  -> seed + Inventory authority
REM
REM Captures each instance_id / code_hash / delegate_key and writes
REM them all into frontend\dev-keys.json. Trunk's copy-file directive
REM picks the file up, the watcher triggers a hot-reload of the tab.
REM
REM NOTE: this Windows variant does NOT run the lockfile-isolation
REM byte-equality gates that the .sh counterpart wires in (relies on
REM bash + cmp + sha256sum). If you publish from Windows, run the gates
REM manually in WSL or another Linux env before pushing: see
REM scripts\check-delegate-byte-equal.sh and check-contract-byte-equal.sh,
REM plus docs\delegate-stability.md for the discipline.
REM
REM Env overrides:
REM   FDEV  -- path to the fdev binary (default: locally-built debug)
REM   WS    -- ws URL of the local node (default: ws://127.0.0.1:7509)

setlocal EnableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
pushd "%SCRIPT_DIR%.." >nul
set "HERE=%CD%"
popd >nul

if "%FDEV%"=="" set "FDEV=%HERE%\..\freenet-core\target\debug\fdev.exe"

if not exist "%FDEV%" (
    echo [dev-publish] fdev not found at: %FDEV%
    echo [dev-publish] build it first: cd %HERE%\..\freenet-core ^&^& cargo build --bin fdev
    exit /b 1
)

REM Per-contract empty initial state. Each is the bincode-serialized
REM Default::default() of the contract's *State struct in shared\.
REM Bincode 1.x uses fixed-int u64 (8 bytes little-endian) for Vec /
REM BTreeMap length, plus 1 byte for the `version: u8` prefix.
REM
REM   presence ContractState  : version(1) + entries(8) + cumulative_damage(8) = 17 bytes
REM   mailbox  MailboxState   : version(1) + entries(8)                        =  9 bytes
REM   guilds   GuildsState    : version(1) + guilds(8)                         =  9 bytes
REM
REM Keep these in sync with `Default for *State` in shared\src\freenet.rs.
set "PRESENCE_STATE=%TEMP%\idle-presence-state-%RANDOM%.bin"
set "MAILBOX_STATE=%TEMP%\idle-mailbox-state-%RANDOM%.bin"
set "GUILDS_STATE=%TEMP%\idle-guilds-state-%RANDOM%.bin"

powershell -NoProfile -Command "[IO.File]::WriteAllBytes('%PRESENCE_STATE%', ([byte[]]@(1) + [byte[]]::new(16)))"
powershell -NoProfile -Command "[IO.File]::WriteAllBytes('%MAILBOX_STATE%',  ([byte[]]@(1) + [byte[]]::new(8)))"
powershell -NoProfile -Command "[IO.File]::WriteAllBytes('%GUILDS_STATE%',   ([byte[]]@(1) + [byte[]]::new(8)))"

REM -- presence-contract --------------------------------------------
call :build_and_publish_contract presence-contract presence_contract "%PRESENCE_STATE%" "presence-contract" CODE_HASH CONTRACT_ID
if errorlevel 1 exit /b 1

REM -- mailbox-contract ---------------------------------------------
call :build_and_publish_contract mailbox-contract mailbox_contract "%MAILBOX_STATE%" "mailbox-contract" MAILBOX_CODE_HASH MAILBOX_ID
if errorlevel 1 exit /b 1

REM -- guilds-contract ----------------------------------------------
call :build_and_publish_contract guilds-contract guilds_contract "%GUILDS_STATE%" "guilds-contract" GUILDS_CODE_HASH GUILDS_ID
if errorlevel 1 exit /b 1

REM -- identity-delegate --------------------------------------------
REM identity-delegate has no initial state -- `fdev publish delegate`
REM emits a `key:` line rather than `Publishing contract ...`.
echo [dev-publish] building identity-delegate
cd /d "%HERE%\identity-delegate"
set "DELEGATE_BUILD_LOG=%TEMP%\idle-delegate-build-%RANDOM%.log"
set "CARGO_TARGET_DIR=%CD%\target"
powershell -NoProfile -Command "& '%FDEV%' build --package-type delegate 2>&1 | Tee-Object -FilePath '%DELEGATE_BUILD_LOG%'"
if errorlevel 1 (
    echo [dev-publish] delegate build failed
    exit /b 1
)
for /f "usebackq delims=" %%H in (`powershell -NoProfile -Command "$c = (Get-Content -Raw '%DELEGATE_BUILD_LOG%') -replace '\[[0-9;]*m',''; $m = [regex]::Matches($c,'code hash: (\S+)'); if($m.Count -gt 0){$m[$m.Count-1].Groups[1].Value}"`) do set "DELEGATE_CODE_HASH=%%H"
if "%DELEGATE_CODE_HASH%"=="" (
    echo [dev-publish] could not parse delegate code hash
    exit /b 1
)

echo [dev-publish] publishing identity-delegate
set "DELEGATE_PUB_LOG=%TEMP%\idle-delegate-pub-%RANDOM%.log"
powershell -NoProfile -Command "& '%FDEV%' publish --code build\freenet\identity_delegate delegate 2>&1 | Tee-Object -FilePath '%DELEGATE_PUB_LOG%'"
if errorlevel 1 (
    echo [dev-publish] delegate publish failed
    exit /b 1
)
for /f "usebackq delims=" %%K in (`powershell -NoProfile -Command "$c = (Get-Content -Raw '%DELEGATE_PUB_LOG%') -replace '\[[0-9;]*m',''; $m = [regex]::Matches($c,'key: ([1-9A-HJ-NP-Za-km-z]{30,})'); if($m.Count -gt 0){$m[$m.Count-1].Groups[1].Value}"`) do set "DELEGATE_KEY=%%K"
if "%DELEGATE_KEY%"=="" (
    echo [dev-publish] could not parse delegate key
    exit /b 1
)

REM -- write dev-keys.json -----------------------------------------
REM Trunk's copy-file directive picks it up and the watcher triggers
REM a hot-reload of the browser tab. Field names must mirror
REM `DevKeys` in frontend\src\main.rs.
set "DEVKEYS=%HERE%\frontend\dev-keys.json"
> "%DEVKEYS%" echo {
>> "%DEVKEYS%" echo   "contract_id_b58": "%CONTRACT_ID%",
>> "%DEVKEYS%" echo   "code_hash_b58": "%CODE_HASH%",
>> "%DEVKEYS%" echo   "delegate_key_b58": "%DELEGATE_KEY%",
>> "%DEVKEYS%" echo   "delegate_code_hash_b58": "%DELEGATE_CODE_HASH%",
>> "%DEVKEYS%" echo   "mailbox_contract_id_b58": "%MAILBOX_ID%",
>> "%DEVKEYS%" echo   "mailbox_code_hash_b58": "%MAILBOX_CODE_HASH%",
>> "%DEVKEYS%" echo   "guilds_contract_id_b58": "%GUILDS_ID%",
>> "%DEVKEYS%" echo   "guilds_code_hash_b58": "%GUILDS_CODE_HASH%"
>> "%DEVKEYS%" echo }

echo.
echo [dev-publish] wrote frontend\dev-keys.json:
type "%DEVKEYS%"
echo.

REM Clean up temp state files. Logs are kept for post-mortem.
del /q "%PRESENCE_STATE%" "%MAILBOX_STATE%" "%GUILDS_STATE%" 2>nul

endlocal & (
    REM Propagate captured values to the caller so dev-watch.bat can
    REM grep dev-keys.json directly -- nothing else to export here.
)
exit /b 0


REM ===================================================================
REM Build + publish a contract crate. Captures code_hash from the
REM build log and instance_id from the publish log; both written to
REM the variables whose NAMES are passed in %5 / %6 (returned via
REM the standard `endlocal & set` trick).
REM   %1: crate dir (under %HERE%)
REM   %2: built artefact name (under build\freenet\<name>)
REM   %3: empty-state file path
REM   %4: human label for logs
REM   %5: var name to receive code_hash
REM   %6: var name to receive instance_id
REM ===================================================================
:build_and_publish_contract
setlocal EnableDelayedExpansion
set "CRATE=%~1"
set "ARTEFACT=%~2"
set "STATE_FILE=%~3"
set "LABEL=%~4"
set "OUT_HASH=%~5"
set "OUT_ID=%~6"

echo [dev-publish] building %LABEL%
cd /d "%HERE%\%CRATE%"

set "BUILD_LOG=%TEMP%\idle-%CRATE%-build-%RANDOM%.log"
set "PUB_LOG=%TEMP%\idle-%CRATE%-pub-%RANDOM%.log"
set "CARGO_TARGET_DIR=%CD%\target"

powershell -NoProfile -Command "& '%FDEV%' build 2>&1 | Tee-Object -FilePath '%BUILD_LOG%'"
if errorlevel 1 (
    echo [dev-publish] %LABEL% build failed
    endlocal
    exit /b 1
)
for /f "usebackq delims=" %%H in (`powershell -NoProfile -Command "$c = (Get-Content -Raw '%BUILD_LOG%') -replace '\[[0-9;]*m',''; $m = [regex]::Matches($c,'code hash: (\S+)'); if($m.Count -gt 0){$m[$m.Count-1].Groups[1].Value}"`) do set "CODE_HASH_LOCAL=%%H"
if "!CODE_HASH_LOCAL!"=="" (
    echo [dev-publish] could not parse %LABEL% code hash
    endlocal
    exit /b 1
)

echo [dev-publish] publishing %LABEL%
powershell -NoProfile -Command "& '%FDEV%' publish --code build\freenet\%ARTEFACT% contract --state '%STATE_FILE%' 2>&1 | Tee-Object -FilePath '%PUB_LOG%'"
if errorlevel 1 (
    echo [dev-publish] %LABEL% publish failed
    endlocal
    exit /b 1
)
for /f "usebackq delims=" %%I in (`powershell -NoProfile -Command "$c = (Get-Content -Raw '%PUB_LOG%') -replace '\[[0-9;]*m',''; $m = [regex]::Matches($c,'Publishing contract ([1-9A-HJ-NP-Za-km-z]{30,})'); if($m.Count -gt 0){$m[$m.Count-1].Groups[1].Value}"`) do set "INSTANCE_LOCAL=%%I"
if "!INSTANCE_LOCAL!"=="" (
    echo [dev-publish] could not parse %LABEL% instance id
    endlocal
    exit /b 1
)

REM Hand the captured values back to the caller's scope.
endlocal & set "%OUT_HASH%=%CODE_HASH_LOCAL%" & set "%OUT_ID%=%INSTANCE_LOCAL%"
exit /b 0
