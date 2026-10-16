$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $projectRoot 'target\keyboard-runtime\release\tartarus-v2be.exe'
$dll = Join-Path $projectRoot 'vendor\interception\v1.0.1\Interception\library\x64\interception.dll'
$testDir = Join-Path $projectRoot ('artifacts\interception\runtime-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $testDir | Out-Null
$configPath = Join-Path $testDir 'config.json'
$runtimeLog = Join-Path $testDir 'runtime.log'
$trace = Join-Path $testDir 'observer.tsv'
$utf8 = New-Object System.Text.UTF8Encoding($false)
function Write-TestConfig([string]$Text) {
    $temporary = Join-Path $testDir 'config.new'
    [IO.File]::WriteAllText($temporary,$Text,$utf8)
    Move-Item -LiteralPath $temporary -Destination $configPath -Force
}
Write-TestConfig '{"schema_version":1,"mappings":{"01":{"type":"key","key":"B"},"02":{"type":"key","key":"B"},"03":{"type":"disabled"}}}'
$observer = Start-Process -FilePath $exe -ArgumentList "verify 90 `"$trace`"" -WindowStyle Normal -RedirectStandardOutput (Join-Path $testDir 'observer.stdout') -RedirectStandardError (Join-Path $testDir 'observer.stderr') -PassThru
$deadline = (Get-Date).AddSeconds(5)
do {
    Start-Sleep -Milliseconds 100
    $ready = (Test-Path -LiteralPath $trace) -and (Select-String -LiteralPath $trace -Pattern '^# start_epoch_us=' -Quiet)
} until ($ready -or (Get-Date) -ge $deadline)
if (-not $ready) { throw 'Observer not ready; runtime not started' }
$runtime = Start-Process -FilePath $exe -ArgumentList "run `"$dll`" `"$configPath`" `"$runtimeLog`" 60" -WindowStyle Hidden -RedirectStandardOutput (Join-Path $testDir 'runtime.stdout') -RedirectStandardError (Join-Path $testDir 'runtime.stderr') -PassThru
$deadline = (Get-Date).AddSeconds(5)
do {
    Start-Sleep -Milliseconds 100
    $ready = (Test-Path -LiteralPath $runtimeLog) -and (Select-String -LiteralPath $runtimeLog -Pattern '^# runtime active' -Quiet)
} until ($ready -or (Get-Date) -ge $deadline)
if (-not $ready) { Get-Content (Join-Path $testDir 'runtime.stderr'); throw 'Runtime did not start' }
Write-Output "STAGE 1 STARTED: 01 and 02 -> B. Evidence: $testDir"
Start-Sleep -Seconds 24
Write-TestConfig '{invalid'
Start-Sleep -Seconds 4
Write-TestConfig '{"schema_version":1,"mappings":{"01":{"type":"key","key":"C"},"02":{"type":"disabled"},"03":{"type":"disabled"}}}'
Start-Sleep -Milliseconds 500
if (-not (Select-String -LiteralPath $runtimeLog -Pattern 'config-applied revision=2' -Quiet)) { throw 'Reload did not apply; check runtime log' }
Write-Output 'STAGE 2 READY: 01 -> C, 02 disabled, configuration reloaded without restart.'
Start-Sleep -Seconds 33
if (-not (Select-String -LiteralPath $runtimeLog -Pattern '^# runtime stopped' -Quiet)) {
    & $exe stop
    throw 'Runtime deadline not confirmed; stop requested. Inspect logs before continuing.'
}
Write-Output 'STAGE 3 READY: runtime stopped; key 01 should now produce native 1.'
Start-Sleep -Seconds 29
Get-Content -LiteralPath $runtimeLog
Get-Content -LiteralPath (Join-Path $testDir 'runtime.stderr')
Get-Content -LiteralPath (Join-Path $testDir 'observer.stderr')
Write-Output "Evidence saved: $testDir"
