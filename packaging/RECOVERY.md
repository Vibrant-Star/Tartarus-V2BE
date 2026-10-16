# Driver removal and recovery

Interception installs keyboard and mouse filter drivers system-wide. Tartarus V2BE's input routing selects the Tartarus only, but the installed driver dependency is broader. Driver changes require explicit action, administrator rights and a manual reboot. Upstream documents testing through Windows 10; this preview has local Windows 11 hardware evidence, not a universal Windows 11 compatibility guarantee. Do not disable Windows security protections to force installation.

Before installation, retain this release folder. The Install helper refuses an existing Interception registration and records the original keyboard/mouse class filters and service/file state in `artifacts/interception/before-install.json`. These records are machine-specific; do not publish them as source or import them wholesale into another system.

For normal removal, exit all remappers, run Uninstall-Interception.cmd as administrator, confirm UNINSTALL, then save work and reboot yourself. Run Driver-status.cmd afterward: check filter registrations, keyboard/mouse services and files, and actual keyboard/mouse operation. A missing file or successful installer exit code alone does not prove complete removal. If files remain locked, another reboot/uninstall pass may be necessary; retain logs for diagnosis.

If input fails, try Windows Safe Mode and the preserved upstream uninstaller if input is available. Windows Recovery Environment and a pre-existing System Restore point may provide recovery if Safe Mode is inaccessible. No restore point is created or guaranteed by these helpers. Encrypted volumes may require your BitLocker recovery key. This installer is not an offline Windows repair tool.

Never delete keyboard.sys/mouse.sys manually while their class filters remain registered. Never clear all UpperFilters/LowerFilters, remove unrelated keyboard/mouse drivers or import a whole registry snapshot blindly. If ordinary recovery is unavailable, obtain targeted help using the actual Windows volume, active control set and preserved baseline.
