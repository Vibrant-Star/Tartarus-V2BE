$ErrorActionPreference='Stop'
$runPath='HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
if(Get-ItemProperty -LiteralPath $runPath -Name 'TartarusV2BE' -ErrorAction SilentlyContinue){
 Remove-ItemProperty -LiteralPath $runPath -Name 'TartarusV2BE'
}
Write-Output 'Tartarus V2BE autostart is disabled. The running app, profiles and Interception driver were not changed.'
