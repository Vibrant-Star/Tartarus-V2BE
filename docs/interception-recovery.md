# Interception rollback and recovery

Prepared before installation. The upstream installer is retained locally under `vendor/interception/v1.0.1/Interception/command line installer/`. Do not delete this directory during the trial.

## Normal rollback

1. Exit any remapping programs. Our current probe does not enable capture filters.
2. Right-click `D:\Apps\Tartarus V2BE\Uninstall-Interception.cmd` and choose **Run as administrator**. It calls the preserved upstream installer with `/uninstall`, captures output and records registry/file state. Alternatively run `scripts\Interception-Driver.ps1 -Action Uninstall` from an administrator PowerShell.
3. Save work and restart Windows yourself.
4. Run `scripts\Interception-Driver.ps1 -Action Verify` and compare its report with `artifacts/interception/before-install.json`. Keyboard/mouse class filters must match the original values, Interception services must be absent, and driver files should be removed. Verify normal keyboard and mouse operation too. A missing driver file alone is not proof of clean removal.
5. If the installer reports in-use files, restart and rerun the same uninstall, then restart and verify again. Upstream issue #80 reports this two-pass removal behavior: https://github.com/oblitum/Interception/issues/80 . This is a reported workaround, not a guarantee.

The launcher does not automatically elevate, reboot, restore registry snapshots, or delete driver files. Do not manually delete `keyboard.sys`/`mouse.sys` while their class filters remain registered; doing so can break input. Preserve unrelated class filters such as kbdclass, mouclass and any pre-existing Razer entries.

## If normal input stops working

Try Windows Safe Mode and run the same preserved upstream uninstaller as administrator, if input is available. If neither keyboard nor mouse is usable, use Windows Recovery Environment and System Restore to a pre-install restore point if available. No restore point is guaranteed or created by these scripts. Drive letters can differ in recovery, and encrypted Windows volumes may require the BitLocker recovery key. Do not assume this launcher works against an offline Windows installation.

If neither route is available, retain the baseline snapshot for targeted recovery assistance. Do not blindly import an entire keyboard/mouse registry class snapshot or delete driver files offline. Offline registry repair needs the actual Windows volume, active control set and original filter values checked first.

## Records

`artifacts/interception/before-install.json` is written once before installation. Additional timestamped reports record class UpperFilters/LowerFilters, Interception service entries, file hashes and installer output. These contain local system details and are excluded from source control. Scripts never reset all filters or uninstall existing Razer drivers.
