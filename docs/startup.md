# Launch, Windows autostart and removal

Double-click **Tartarus V2BE.exe** in the project folder. It starts one background process with the existing tray icon and no console window. Open configuration from the tray. Opening the executable again reuses the existing local editor; it does not restart capture, reapply settings or create a second resident process.

In the editor's left sidebar, expand **Launch settings**:

- **Start with Windows** registers this executable for your Windows account at sign-in.
- **Start remapping when app launches** activates the saved data/config.json whenever a new app instance starts. If the device is missing or keys/buttons are held, the runtime uses its existing explicit recovery/waiting state.
- **Open configuration on manual launch** opens the default browser on an ordinary manual launch. Windows sign-in always passes --background, so it never deliberately opens a browser.

Save launch settings applies the launch preferences immediately for future launches. It does not start or stop the current runtime. All three options default to off and were left off after validation. Stop pauses remapping for this session; it does not disable the next launch's automatic-remapping preference. Exit releases owned outputs and closes the app. Closing the browser leaves the app running.

## Storage and Windows integration

auto_remap and open_browser live in human-readable data/application.json, separate from device profiles and Synapse imports. The file has schema_version 1 and uses same-directory atomic replacement. Startup registration is read from Windows rather than trusting a duplicate Boolean in the file. Authenticated settings requests include a revision that rejects stale edits from another tab. A registry update failure attempts to restore the previous preferences and reports the result.

The only sign-in registration is REG_SZ **TartarusV2BE** under **HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run**. It points to the exact executable and application root with quoted arguments and --background. It does not use a scheduled task, Windows service, machine-wide key, helper process, elevation request or kernel-driver modification. It refuses a conflicting entry at another location until you disable that entry first. Startup commands over Windows' documented 260-character limit are rejected.

Windows can delay sign-in startup, and Startup Apps settings or policy can disable a registered entry. The UI reports registration, not proof that Windows will execute it at the next sign-in. Registration/removal and equivalent launch behavior were tested; an actual sign-out/sign-in has not been performed. See [Microsoft's Run-key documentation](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys).

## Portable layout and rebuilding

For the current development checkout, the root executable finds the existing vendor Interception DLL and data directory. Build with:

~~~powershell
cargo build --release --offline --target-dir target/startup
Copy-Item -LiteralPath target/startup/release/tartarus-v2be.exe -Destination 'Tartarus V2BE.exe' -Force
~~~

Exit the app before replacing its executable. The native launcher can also run in a portable folder with interception.dll beside the executable; it creates data/config.json with native mappings only when that file does not already exist. Keep this folder in a stable writable location. The executable embeds the web UI and bundled-font license; distribution must also include project/dependency license notices. This increment does not supply an installer or install the Interception kernel driver. That existing driver dependency remains required and separately documented.

The optional scripts/Open-UI.ps1 now invokes the native launcher with --open (or --background for -NoBrowser). Native commands are launch [--root PATH] [--background|--open] and disable-autostart. The release executable is a Windows GUI-subsystem application; diagnostic CLI output should be redirected/captured when needed. Normal launch errors display a message box.

## Disable or remove

1. Uncheck **Start with Windows** and Save launch settings. Optionally uncheck automatic remapping too.
2. Use **Exit** from the tray and wait for capture cleanup.
3. Keep data if you want to preserve settings/profiles. You can then move or remove the portable app folder.

If the UI cannot be opened, run scripts/Remove-Autostart.ps1 or invoke the executable with disable-autostart. The removal script is idempotent and deletes only the named per-user startup value; it does not stop the app, delete profiles or change any driver. Disable autostart before moving the executable; then register it again from the new location if desired.

Removing this application does not uninstall Interception. Its existing separately documented clean-uninstall procedure remains available; driver removal requires administrator approval/reboot and was not performed by this feature.

## Validation — 2026-10-16

46 software tests pass and one Explorer lifecycle test remains opt-in. Added coverage includes strict settings, atomic preferences replacement/corruption rejection and real Windows command-line parsing of spaces, Unicode and drive-root trailing slashes. Formatting, Clippy with warnings denied, JavaScript syntax and optimized release build pass.

A live authenticated round trip temporarily enabled and removed this application's HKCU Run value, verified the registered command matched the current executable/root, rejected a stale settings update with HTTP 409, and restored all options off. A duplicate quiet launch exited successfully with the original server process unchanged. A fresh quiet launch with auto_remap enabled reached active/applied capture; it was then stopped and the preference turned off. Device configuration remained byte-for-byte unchanged.

A no-argument launch without redirected console handles started the editor successfully with all launch options off. The executable's PE subsystem is 2 (Windows GUI), and the removal script completed idempotently. The UI displays all three unchecked controls, saves them successfully and leaves remapping stopped. Evidence is under artifacts/startup-test. No actual Windows sign-out/sign-in or restart was induced.
