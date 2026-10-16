@echo off
rem Run as administrator. No reboot is performed by this launcher.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\Interception-Driver.ps1" -Action Uninstall
pause
