param([ValidateSet('momentary','cycle','modifier')][string]$Mode = 'momentary')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $projectRoot 'target\layer-test\release\tartarus-v2be.exe'
$dll = Join-Path $projectRoot 'vendor\interception\v1.0.1\Interception\library\x64\interception.dll'
$testDir = Join-Path $projectRoot ('artifacts\interception\layers-' + $Mode + '-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $testDir | Out-Null
$configPath = Join-Path $testDir 'config.json'
$config = Get-Content -LiteralPath (Join-Path $projectRoot 'config.layers.example.json') -Raw | ConvertFrom-Json
if ($Mode -eq 'cycle') { $config.thumb = @{ mode='cycle'; layers=@(0,1,2) } }
if ($Mode -eq 'modifier') { $config.thumb = @{ mode='modifier'; key='LeftAlt' } }
[IO.File]::WriteAllText($configPath,($config | ConvertTo-Json -Depth 8),(New-Object System.Text.UTF8Encoding($false)))
$runtimeLog = Join-Path $testDir 'runtime.log'
$trace = Join-Path $testDir 'observer.tsv'
function Wait-Log([string]$File,[string]$Pattern) {
    $deadline = (Get-Date).AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 100
        $ready = (Test-Path -LiteralPath $File) -and (Select-String -LiteralPath $File -Pattern $Pattern -Quiet)
    } until ($ready -or (Get-Date) -ge $deadline)
    if (-not $ready) { throw "Expected status not received: $Pattern" }
}
$observer = Start-Process -FilePath $exe -ArgumentList "verify 105 `"$trace`"" -WindowStyle Normal -RedirectStandardOutput (Join-Path $testDir 'observer.stdout') -RedirectStandardError (Join-Path $testDir 'observer.stderr') -PassThru
Wait-Log $trace '^# start_epoch_us='
$runtime = Start-Process -FilePath $exe -ArgumentList "run `"$dll`" `"$configPath`" `"$runtimeLog`" 90" -WindowStyle Hidden -RedirectStandardOutput (Join-Path $testDir 'runtime.stdout') -RedirectStandardError (Join-Path $testDir 'runtime.stderr') -PassThru
Wait-Log $runtimeLog '^# runtime active'
Write-Output "LAYER TEST READY: mode=$Mode. Evidence: $testDir"
Start-Sleep -Seconds 50
Start-Sleep -Seconds 20
& $exe stop
if ($LASTEXITCODE -ne 0) { throw 'Stop request failed; 90-second safety deadline remains active' }
Wait-Log $runtimeLog '^# runtime stopped'
Write-Output 'LAYER TEST STOPPED: native mappings restored; release all controls.'
Start-Sleep -Seconds 32
Get-Content -LiteralPath $runtimeLog
Get-Content -LiteralPath (Join-Path $testDir 'runtime.stderr')
Get-Content -LiteralPath (Join-Path $testDir 'observer.stderr')
Write-Output "Evidence saved: $testDir"
