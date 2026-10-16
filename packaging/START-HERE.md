# Tartarus V2BE — Windows 11 x64 preview

Extract the entire folder somewhere writable, such as your user Apps folder. Keep its files together. This is an experimental portable release; no installer for Tartarus V2BE itself is needed.

## First use

1. If Interception is already installed and working, skip driver installation. Do not reinstall a shared driver just for this app.
2. Otherwise read `THIRD-PARTY.md` and `RECOVERY.md`, exit remappers, then right-click `Install-Interception.cmd` and choose **Run as administrator**. The helper explains the system-wide keyboard/mouse filter dependency and requires typing INSTALL. It records the original filter state and uses the pinned upstream installer. Save your work and reboot manually afterward.
3. Double-click `Tartarus V2BE.exe`. It starts quietly with a tray icon. Open configuration from the tray, or use `Open configuration.cmd`. A manual relaunch while the app is running also opens configuration.
4. Configure mappings at http://127.0.0.1:17832/. Remapping starts only when requested. Windows autostart and automatic remapping are off initially. Lighting management is off initially.

The target is **Razer Tartarus V2, USB 1532:022B**, not Tartarus Pro. Synapse is not required. The browser can be closed while remapping continues. Configuration binds to 127.0.0.1 only. Stop restores native input; Exit closes the background app. Ordinary app use does not install drivers or request administrator access. Driver access on other Windows installations remains an acceptance item; do not assume every security configuration is supported.

## Settings and updates

Your settings and library profiles live in `data/`. Back up that folder. Release archives contain fresh defaults, never another user's profiles. For an update, exit the app, extract the new release separately, and copy your backed-up `data/` into it. Re-enable autostart from the new location if needed. Never replace a running executable. Read `docs/` for feature limits and known acceptance gaps.

## Remove the app

Disable Start with Windows in App settings and save, then Stop and Exit. Back up `data/` and delete the extracted folder when ready. If the UI is unavailable, `Disable-autostart.cmd` removes the named TartarusV2BE startup entry after confirmation; it may point to another copy, so review its displayed value first. Deleting the portable folder does not remove Interception.

## Remove Interception separately

Only do this if no other application needs it. Exit all remappers, right-click `Uninstall-Interception.cmd` → Run as administrator, and type UNINSTALL. Reboot manually and use `Driver-status.cmd` to review state. Preserve the folder until removal is verified. See `RECOVERY.md` if input fails. No helper automatically reboots Windows or resets unrelated class filters.

## Licenses and source

Project-authored code is MIT licensed. Interception is a separately licensed dependency with binary driver/installer components; the full stack is not claimed to be entirely open source. See `THIRD-PARTY.md`, `licenses/` and `source/`. Our project source is offered separately on GitHub and in the matching source download; the dependency library source remains included. The DLL is separate and replaceable with an API-compatible library. Hashes in `manifest.sha256` describe this package; they detect accidental changes but are not an independent signature or trust anchor. Build instructions are in `docs/packaging.md`.
