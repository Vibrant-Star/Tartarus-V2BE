[CmdletBinding()]
param([ValidatePattern('^[A-Za-z0-9._-]+$')][string]$OutputName=('Tartarus-V2BE-0.1.0-preview-win11-x64-'+[DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss')+'Z'))
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
if([Environment]::OSVersion.Platform -ne 'Win32NT'){throw 'Build this Windows release on Windows.'}
$output=Join-Path $root ('dist/'+$OutputName)
$zipPath=$output+'.zip'
$applicationSource=$output+'-source.zip'
if((Test-Path -LiteralPath $output) -or (Test-Path -LiteralPath $zipPath) -or (Test-Path -LiteralPath $applicationSource)){throw 'Output already exists; choose a new OutputName. Nothing overwritten.'}
& "$PSScriptRoot/Prepare-PackageInputs.ps1"
if($LASTEXITCODE -and $LASTEXITCODE -ne 0){throw 'Packaging input verification failed'}
$previousFlags=$env:CARGO_ENCODED_RUSTFLAGS
try {
 $flags=@("--remap-path-prefix=$root=.","--remap-path-prefix=$env:USERPROFILE=C:/build/user") -join [char]31
 $env:CARGO_ENCODED_RUSTFLAGS=if($previousFlags){$previousFlags+[char]31+$flags}else{$flags}
 & cargo build --locked --release --target x86_64-pc-windows-msvc --target-dir target/package
 if($LASTEXITCODE -ne 0){throw 'Release build failed'}
} finally {$env:CARGO_ENCODED_RUSTFLAGS=$previousFlags}
$metadata=(& cargo metadata --locked --format-version 1 | Out-String | ConvertFrom-Json)
if($LASTEXITCODE -ne 0){throw 'Cargo metadata failed'}
foreach($dir in @('data/profiles','scripts','licenses/Interception','licenses/Inter-font','licenses/Rust','source','docs','vendor/interception/v1.0.1/Interception/command line installer')){
 New-Item -ItemType Directory -Force -Path (Join-Path $output $dir) | Out-Null
}
Copy-Item -LiteralPath target/package/x86_64-pc-windows-msvc/release/tartarus-v2be.exe -Destination (Join-Path $output 'Tartarus V2BE.exe')
$binary=[IO.File]::ReadAllBytes((Join-Path $output 'Tartarus V2BE.exe'))
foreach($encoding in @([Text.Encoding]::ASCII,[Text.Encoding]::Unicode)){
 $text=$encoding.GetString($binary)
 if($text.Contains($env:USERPROFILE)){throw 'Build-user path remains in executable; refusing to package it.'}
}
[IO.File]::WriteAllText((Join-Path $output 'data/config.json'),'{"schema_version":1,"mappings":{}}'+[Environment]::NewLine)
[IO.File]::WriteAllText((Join-Path $output 'data/application.json'),'{"schema_version":1,"auto_remap":false,"open_browser":false,"show_tray":true}'+[Environment]::NewLine)
[IO.File]::WriteAllText((Join-Path $output 'data/lighting.json'),'{"schema_version":1,"managed":false,"effect":"spectrum","color":[0,255,80],"brightness":75,"speed":2,"direction":1}'+[Environment]::NewLine)
Copy-Item -LiteralPath LICENSE -Destination (Join-Path $output 'LICENSE.txt')
Copy-Item -LiteralPath packaging/START-HERE.md,packaging/RECOVERY.md -Destination $output
Copy-Item -Path docs/*.md -Destination (Join-Path $output 'docs')
Copy-Item -LiteralPath web/fonts/LICENSE.txt -Destination (Join-Path $output 'licenses/Inter-font/LICENSE.txt')
Copy-Item -LiteralPath artifacts/package-inputs/GPL-3.0.txt -Destination (Join-Path $output 'licenses/Interception/GPL-3.0.txt')
Copy-Item -LiteralPath artifacts/package-inputs/Interception-v1.0.1-source.zip -Destination (Join-Path $output 'source')
Add-Type -AssemblyName System.IO.Compression.FileSystem
$upstream=[IO.Compression.ZipFile]::OpenRead((Join-Path $root 'vendor/interception/Interception.zip'))
try {
 foreach($item in @(
  @('Interception/library/x64/interception.dll','interception.dll'),
  @('Interception/command line installer/install-interception.exe','vendor/interception/v1.0.1/Interception/command line installer/install-interception.exe'),
  @('Interception/licenses/non-commercial-usage/LGPL 3.0.txt','licenses/Interception/LGPL-3.0.txt')
 )){
  $entry=$upstream.GetEntry($item[0]);if(!$entry){throw "Missing upstream asset $($item[0])"}
  [IO.Compression.ZipFileExtensions]::ExtractToFile($entry,(Join-Path $output $item[1]))
 }
} finally {$upstream.Dispose()}
$installerHash='E137863A79DA797F08E7A137280FF2A123809044A888FD75CE9C973198915ABE'
if((Get-FileHash -LiteralPath (Join-Path $output 'vendor/interception/v1.0.1/Interception/command line installer/install-interception.exe')).Hash -ne $installerHash){throw 'Installer hash mismatch'}
if((Get-FileHash -LiteralPath (Join-Path $output 'interception.dll')).Hash -ne 'AB88164C11B1B48488772D4C3BFAA4509D5B0AE9DBC5A691DC4F96F0260443C8'){throw 'DLL hash mismatch'}
$source=[IO.Compression.ZipFile]::OpenRead((Join-Path $root 'artifacts/package-inputs/Interception-v1.0.1-source.zip'))
try{
 $readme=$source.Entries | Where-Object {$_.FullName -match '^[^/]+/README.md$'} | Select-Object -First 1
 if(!$readme){throw 'Missing upstream license statement'}
 [IO.Compression.ZipFileExtensions]::ExtractToFile($readme,(Join-Path $output 'licenses/Interception/UPSTREAM-README.md'))
}finally{$source.Dispose()}
$notices=@('# Third-party components','','Original Tartarus V2BE code: MIT; see LICENSE.txt.','','Interception v1.0.1, Francisco Lopes: upstream states LGPLv3 library/source and non-commercial redistribution permission for related driver/installer binaries when used solely through its library/API. See licenses/Interception/UPSTREAM-README.md and GPL/LGPL texts. Commercial terms must be checked separately; free distribution does not relicense this dependency. Driver/installer source is not supplied by the public upstream archive.','','Official binary archive: https://github.com/oblitum/Interception/releases/download/v1.0.1/Interception.zip','Pinned archive SHA256: AD038963D6413055765128B0B931F6E765147C9916DBA79E65D872B261F9AF10','Corresponding public library source: source/Interception-v1.0.1-source.zip (upstream tag v1.0.1). The separate DLL may be replaced by an API-compatible modified library; no prohibition on reverse engineering to debug library modifications is imposed.','','Inter variable font: SIL Open Font License 1.1; see licenses/Inter-font/.','','Rust dependencies (including build/target-conditional dependencies); exact notices are preserved under licenses/Rust/:','')
foreach($package in ($metadata.packages | Where-Object {$_.source} | Sort-Object name)){
 $dir=Split-Path -Parent $package.manifest_path
 $files=@(Get-ChildItem -LiteralPath $dir -File | Where-Object {$_.Name -match '^(licen[sc]e|copying|copyright|notice|authors)'})
 if(!$files.Count){throw "No dependency notices found for $($package.name)"}
 $destination=Join-Path $output ('licenses/Rust/'+$package.name+'-'+$package.version)
 New-Item -ItemType Directory -Force -Path $destination | Out-Null
 foreach($file in $files){Copy-Item -LiteralPath $file.FullName -Destination $destination}
 $notices+=('- '+$package.name+' '+$package.version+': '+$package.license)
}
$notices | Set-Content -LiteralPath (Join-Path $output 'THIRD-PARTY.md') -Encoding UTF8
$driver=Get-Content -LiteralPath scripts/Interception-Driver.ps1 -Raw
$anchor='$before = Get-InputState'
if(!$driver.Contains($anchor)){throw 'Driver helper template changed; review packaging'}
$confirmation=@'
$expectedInstallerHash='E137863A79DA797F08E7A137280FF2A123809044A888FD75CE9C973198915ABE'
if((Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash -ne $expectedInstallerHash){throw 'Upstream installer hash mismatch; refusing execution.'}
Write-Output 'Interception changes system-wide keyboard/mouse filter drivers. Administrator access and a manual reboot are required.'
Write-Output 'Exit Tartarus V2BE and all other remappers first. Uninstalling also affects any other application that uses Interception.'
$confirmation=$Action.ToUpperInvariant()
if((Read-Host "Type $confirmation to proceed") -cne $confirmation){Write-Output 'Cancelled. No driver changes made.';exit 0}
$before = Get-InputState
'@
$driver.Replace($anchor,$confirmation) | Set-Content -LiteralPath (Join-Path $output 'scripts/Interception-Driver.ps1') -Encoding UTF8
Copy-Item -LiteralPath packaging/Disable-Autostart.ps1 -Destination (Join-Path $output 'scripts')
foreach($action in @('Install','Uninstall','Verify')){
 $name=if($action -eq 'Verify'){'Driver-status.cmd'}else{$action+'-Interception.cmd'}
 $cmd="@echo off`r`npowershell.exe -NoProfile -ExecutionPolicy Bypass -File `"%~dp0scripts\Interception-Driver.ps1`" -Action $action`r`npause`r`n"
 [IO.File]::WriteAllText((Join-Path $output $name),$cmd)
}
[IO.File]::WriteAllText((Join-Path $output 'Disable-autostart.cmd'),"@echo off`r`npowershell.exe -NoProfile -ExecutionPolicy Bypass -File `"%~dp0scripts\Disable-Autostart.ps1`"`r`npause`r`n")
[IO.File]::WriteAllText((Join-Path $output 'Open configuration.cmd'),'@echo off'+"`r`n"+'start "" "%~dp0Tartarus V2BE.exe" launch --open --root "%~dp0."'+"`r`n")
$archive=[IO.Compression.ZipFile]::Open($applicationSource,[IO.Compression.ZipArchiveMode]::Create)
try{
 $files=@('Cargo.toml','Cargo.lock','LICENSE','README.md','CONTRIBUTING.md','config.example.json','config.layers.example.json','config.wheel.example.json','.gitignore') | ForEach-Object {Get-Item -LiteralPath (Join-Path $root $_)}
 foreach($dir in @('src','web','scripts','packaging','docs','.github','assets')){$files+=Get-ChildItem -LiteralPath (Join-Path $root $dir) -File -Recurse}
 foreach($file in $files){
  $relative=$file.FullName.Substring($root.Length+1).Replace('\','/')
  [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive,$file.FullName,$relative,[IO.Compression.CompressionLevel]::Optimal) | Out-Null
 }
}finally{$archive.Dispose()}
$manifest=Get-ChildItem -LiteralPath $output -File -Recurse | Sort-Object FullName | ForEach-Object {
 (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash+'  '+$_.FullName.Substring($output.Length+1).Replace('\','/')
}
$manifest | Set-Content -LiteralPath (Join-Path $output 'manifest.sha256') -Encoding ASCII
[IO.Compression.ZipFile]::CreateFromDirectory($output,$zipPath,[IO.Compression.CompressionLevel]::Optimal,$true)
$hash=(Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
[IO.File]::WriteAllText($zipPath+'.sha256',$hash+'  '+[IO.Path]::GetFileName($zipPath)+[Environment]::NewLine)
[IO.File]::WriteAllText($applicationSource+'.sha256',(Get-FileHash -LiteralPath $applicationSource -Algorithm SHA256).Hash+'  '+[IO.Path]::GetFileName($applicationSource)+[Environment]::NewLine)
Write-Output "Project source (separate download): $applicationSource"
Write-Output "Portable release: $zipPath"
Write-Output "SHA256: $hash"
Write-Output 'No drivers installed, removed, loaded or modified. No personal settings or captures included.'
