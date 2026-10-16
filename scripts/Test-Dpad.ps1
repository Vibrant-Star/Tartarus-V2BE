$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $projectRoot 'target\dpad-test\release\tartarus-v2be.exe'
$dll = Join-Path $projectRoot 'vendor\interception\v1.0.1\Interception\library\x64\interception.dll'
$testDir = Join-Path $projectRoot ('artifacts\interception\dpad-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $testDir | Out-Null
$configPath = Join-Path $testDir 'config.json'
$runtimeLog = Join-Path $testDir 'runtime.log'
$trace = Join-Path $testDir 'observer.tsv'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$configText = '{"schema_version":1,"mappings":{"dpad_up":{"type":"key","key":"A"},"dpad_down":{"type":"key","key":"B"},"dpad_left":{"type":"key","key":"C"},"dpad_right":{"type":"key","key":"D"}}}'
[IO.File]::WriteAllText($configPath,$configText,$utf8)
function Wait-Log([string]$File,[string]$Pattern) {
    $deadline = (Get-Date).AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 100
        $ready = (Test-Path -LiteralPath $File) -and (Select-String -LiteralPath $File -Pattern $Pattern -Quiet)
    } until ($ready -or (Get-Date) -ge $deadline)
    if (-not $ready) { throw "Expected status not received: $Pattern" }
}
$observer = Start-Process -FilePath $exe -ArgumentList "verify 110 `"$trace`"" -WindowStyle Normal -RedirectStandardOutput (Join-Path $testDir 'observer.stdout') -RedirectStandardError (Join-Path $testDir 'observer.stderr') -PassThru
Wait-Log $trace '^# start_epoch_us='
$runtime = Start-Process -FilePath $exe -ArgumentList "run `"$dll`" `"$configPath`" `"$runtimeLog`" 80" -WindowStyle Hidden -RedirectStandardOutput (Join-Path $testDir 'runtime.stdout') -RedirectStandardError (Join-Path $testDir 'runtime.stderr') -PassThru
Wait-Log $runtimeLog '^# runtime active'
Write-Output "D-PAD LETTER TEST STARTED: distinct A/B/C/D outputs. Evidence: $testDir"
Start-Sleep -Seconds 55
Start-Sleep -Seconds 10
& $exe stop
if ($LASTEXITCODE -ne 0) { throw 'Stop request failed; 80-second safety limit remains active' }
Wait-Log $runtimeLog '^# runtime stopped'
Write-Output 'D-PAD RESTORATION READY: runtime stopped; move/release the D-pad to check native arrow events.'
Start-Sleep -Seconds 42
Get-Content -LiteralPath $runtimeLog
Get-Content -LiteralPath (Join-Path $testDir 'runtime.stderr')
Get-Content -LiteralPath (Join-Path $testDir 'observer.stderr')
Write-Output "Evidence saved: $testDir"
