$ErrorActionPreference='Stop'
$path='HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$entry=Get-ItemProperty -LiteralPath $path -Name TartarusV2BE -ErrorAction SilentlyContinue
if(!$entry){Write-Output 'No TartarusV2BE startup entry exists.';exit 0}
Write-Output ('Registered command: '+$entry.TartarusV2BE)
Write-Output 'This may refer to another copy of Tartarus V2BE. The running app and drivers will not be changed.'
if((Read-Host 'Type DISABLE to remove this named startup entry') -cne 'DISABLE'){Write-Output 'Cancelled.';exit 0}
Remove-ItemProperty -LiteralPath $path -Name TartarusV2BE
Write-Output 'Autostart disabled. Stop/Exit the app before deleting its folder.'
