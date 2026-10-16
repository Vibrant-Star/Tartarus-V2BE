param([switch]$NoBrowser)
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
$exe=Join-Path $projectRoot 'Tartarus V2BE.exe'
if(!(Test-Path -LiteralPath $exe)){$exe=Join-Path $projectRoot 'target/ui/release/tartarus-v2be.exe'}
if(!(Test-Path -LiteralPath $exe)){throw 'Build and copy the release executable first; see docs/startup.md.'}
$mode=if($NoBrowser){'--background'}else{'--open'}
$process=Start-Process -FilePath $exe -ArgumentList @('launch','--root',('"'+$projectRoot+'"'),$mode) -WindowStyle Hidden -PassThru
Write-Output 'Tartarus V2BE launched. Use its tray icon for configuration or Stop. Automatic remapping follows Launch settings.'
