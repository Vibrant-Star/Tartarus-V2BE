[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Snapshot', 'Install', 'Uninstall', 'Verify')]
    [string]$Action
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$recordRoot = Join-Path $projectRoot 'artifacts\interception'
$installer = Join-Path $projectRoot 'vendor\interception\v1.0.1\Interception\command line installer\install-interception.exe'
$classes = @('{4D36E96B-E325-11CE-BFC1-08002BE10318}', '{4D36E96F-E325-11CE-BFC1-08002BE10318}')
New-Item -ItemType Directory -Path $recordRoot -Force | Out-Null

function Get-InputState {
    $classState = foreach ($classId in $classes) {
        $key = Get-ItemProperty -LiteralPath "HKLM:\SYSTEM\CurrentControlSet\Control\Class\$classId"
        [ordered]@{ Class = $classId; UpperFilters = @($key.UpperFilters); LowerFilters = @($key.LowerFilters) }
    }
    $serviceState = foreach ($serviceName in @('keyboard', 'mouse')) {
        $serviceKey = "HKLM:\SYSTEM\CurrentControlSet\Services\$serviceName"
        $entry = if (Test-Path -LiteralPath $serviceKey) { Get-ItemProperty -LiteralPath $serviceKey } else { $null }
        $driverFile = Join-Path $env:SystemRoot "System32\drivers\$serviceName.sys"
        [ordered]@{
            Name = $serviceName; Exists = ($null -ne $entry)
            ImagePath = $entry.ImagePath; Start = $entry.Start; Type = $entry.Type
            FileExists = (Test-Path -LiteralPath $driverFile)
            FileHash = $(if (Test-Path -LiteralPath $driverFile) { (Get-FileHash -LiteralPath $driverFile -Algorithm SHA256).Hash } else { $null })
        }
    }
    [ordered]@{ Timestamp = (Get-Date).ToString('o'); Classes = @($classState); Services = @($serviceState) }
}

if ($Action -in @('Snapshot', 'Verify')) {
    $state = Get-InputState
    $destination = Join-Path $recordRoot "$($Action.ToLower())-$(Get-Date -Format 'yyyyMMdd-HHmmss').json"
    $state | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $destination -Encoding UTF8
    $state | ConvertTo-Json -Depth 8
    Write-Output "Saved: $destination"
    exit 0
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this script from an administrator PowerShell window.'
}
if (-not (Test-Path -LiteralPath $installer)) { throw "Missing upstream installer: $installer" }
$before = Get-InputState
if ($Action -eq 'Install') {
    # Preserve the baseline once. Never overwrite it with an installed-state snapshot.
    $baselinePath = Join-Path $recordRoot 'before-install.json'
    if (Test-Path -LiteralPath $baselinePath) { throw 'Baseline already exists. Review the prior trial before installing again.' }
    foreach ($entry in $before.Services) {
        if ($entry.Exists -or $entry.FileExists) { throw "Existing $($entry.Name) service/file; refusing to overwrite a prior driver." }
    }
    foreach ($entry in $before.Classes) {
        if (@($entry.UpperFilters + $entry.LowerFilters) | Where-Object { $_ -in @('keyboard', 'mouse') }) {
            throw 'Existing Interception filter registration; refusing to overwrite a prior trial.'
        }
    }
    $before | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $baselinePath -Encoding UTF8
}
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$logPrefix = Join-Path $recordRoot "$($Action.ToLower())-$stamp"
$installerHash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
$before | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath "$logPrefix-before.json" -Encoding UTF8
$process = Start-Process -FilePath $installer -ArgumentList "/$($Action.ToLower())" -WorkingDirectory (Split-Path -Parent $installer) -WindowStyle Hidden -Wait -PassThru -RedirectStandardOutput "$logPrefix.stdout.txt" -RedirectStandardError "$logPrefix.stderr.txt"
Get-InputState | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath "$logPrefix-after.json" -Encoding UTF8
[ordered]@{ Action = $Action; ExitCode = $process.ExitCode; InstallerSHA256 = $installerHash; Timestamp = (Get-Date).ToString('o') } | ConvertTo-Json | Set-Content -LiteralPath "$logPrefix-result.json" -Encoding UTF8
Get-Content -LiteralPath "$logPrefix.stdout.txt"
Get-Content -LiteralPath "$logPrefix.stderr.txt"
Write-Output "Installer exit: $($process.ExitCode). Logs: $logPrefix. No reboot performed."
exit $process.ExitCode
