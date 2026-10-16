$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $projectRoot 'target\keyboard-runtime\release\tartarus-v2be.exe'
$dll = Join-Path $projectRoot 'vendor\interception\v1.0.1\Interception\library\x64\interception.dll'
$testDir = Join-Path $projectRoot ('artifacts\interception\reload-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $testDir | Out-Null
$configPath = Join-Path $testDir 'config.json'
$runtimeLog = Join-Path $testDir 'runtime.log'
$trace = Join-Path $testDir 'observer.tsv'
$utf8 = New-Object System.Text.UTF8Encoding($false)
function Write-Config([string]$Key) {
    $text = '{"schema_version":1,"mappings":{"01":{"type":"key","key":"' + $Key + '"},"02":{"type":"disabled"}}}'
    $temporary = Join-Path $testDir 'config.new'
    [IO.File]::WriteAllText($temporary,$text,$utf8)
    Move-Item -LiteralPath $temporary -Destination $configPath -Force
}
function Wait-Log([string]$File,[string]$Pattern) {
    $deadline = (Get-Date).AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 100
        $ready = (Test-Path -LiteralPath $File) -and (Select-String -LiteralPath $File -Pattern $Pattern -Quiet)
    } until ($ready -or (Get-Date) -ge $deadline)
    if (-not $ready) { throw "Expected status not received: $Pattern" }
}
Write-Config 'B'
$observer = Start-Process -FilePath $exe -ArgumentList "verify 80 `"$trace`"" -WindowStyle Normal -RedirectStandardOutput (Join-Path $testDir 'observer.stdout') -RedirectStandardError (Join-Path $testDir 'observer.stderr') -PassThru
Wait-Log $trace '^# start_epoch_us='
$runtime = Start-Process -FilePath $exe -ArgumentList "run `"$dll`" `"$configPath`" `"$runtimeLog`" 60" -WindowStyle Hidden -RedirectStandardOutput (Join-Path $testDir 'runtime.stdout') -RedirectStandardError (Join-Path $testDir 'runtime.stderr') -PassThru
Wait-Log $runtimeLog '^# runtime active'
Write-Config 'C'
Wait-Log $runtimeLog 'config-applied revision=2'
Write-Output "LIVE C TEST READY: 01 -> C after reload, 02 disabled. Evidence: $testDir"
Start-Sleep -Seconds 40
& $exe stop
if ($LASTEXITCODE -ne 0) { throw 'Graceful stop request failed; 60-second safety deadline remains active' }
Wait-Log $runtimeLog '^# runtime stopped'
Write-Output 'RESTORATION TEST READY: runtime stopped via stop command; press/release Tartarus 01 for native 1.'
Start-Sleep -Seconds 36
Get-Content -LiteralPath $runtimeLog
Get-Content -LiteralPath (Join-Path $testDir 'runtime.stderr')
Get-Content -LiteralPath (Join-Path $testDir 'observer.stderr')
Write-Output "Evidence saved: $testDir"
