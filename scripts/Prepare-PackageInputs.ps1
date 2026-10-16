[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$dest=Join-Path $root 'artifacts/package-inputs'
New-Item -ItemType Directory -Force -Path $dest | Out-Null
$inputs=@(
 @{Path='vendor/interception/Interception.zip';Url='https://github.com/oblitum/Interception/releases/download/v1.0.1/Interception.zip';Hash='AD038963D6413055765128B0B931F6E765147C9916DBA79E65D872B261F9AF10'},
 @{Path='artifacts/package-inputs/Interception-v1.0.1-source.zip';Url='https://codeload.github.com/oblitum/Interception/zip/refs/tags/v1.0.1';Hash='43E6A4F71085C8797DF3EA1DF40CBDA72530546F45B697944A237A1A8CD3EAA8'},
 @{Path='artifacts/package-inputs/GPL-3.0.txt';Url='https://www.gnu.org/licenses/gpl-3.0.txt';Hash='3972DC9744F6499F0F9B2DBF76696F2AE7AD8AF9B23DDE66D6AF86C9DFB36986'}
)
foreach($item in $inputs){
 $path=Join-Path $root $item.Path
 New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
 if(!(Test-Path -LiteralPath $path)){
  $temp=$path+'.download'
  Invoke-WebRequest $item.Url -OutFile $temp
  if((Get-FileHash -LiteralPath $temp -Algorithm SHA256).Hash -ne $item.Hash){throw "Checksum mismatch: $($item.Url). Download retained for review; nothing executed."}
  Move-Item -LiteralPath $temp -Destination $path
 }
 if((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $item.Hash){throw "Checksum mismatch: $path. Refusing to use it."}
}
Write-Output 'Pinned packaging inputs verified. No installer or driver was executed.'
