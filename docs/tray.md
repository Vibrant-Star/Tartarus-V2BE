# Basic Windows tray controls

**App settings → Show app in tray** controls icon visibility. Save app settings to apply it immediately; the preference persists in `data/application.json` as `show_tray`. Hiding the tray releases its icon/window/thread while the server, remapping and lighting continue independently. Older settings files default to showing the icon. A manual relaunch of an already-running executable opens configuration even when the manual-launch browser preference is off; background/autostart launches remain quiet. The browser header still provides Stop and Exit while the tray is hidden.

The `ui` command now creates a native Windows notification icon in the **same process** as the local server and input runtime. The existing `scripts/Open-UI.ps1` launcher starts it hidden. The green T may initially be in Windows' hidden-icons overflow; pinning it is optional and the application does not change taskbar preferences.

- Double-click the icon, or choose **Open configuration**, to open `http://127.0.0.1:17832/` in your default browser.
- **Start remapping** uses the saved active JSON, not an unapplied browser draft. It starts until explicitly stopped; the browser's 60-second test selector applies only when starting from the browser.
- **Stop remapping** requests the existing graceful release/filter cleanup. Start and Stop are enabled according to the observed runtime state when the menu opens. Startup transitions can briefly reject another action; errors are shown rather than silently ignored.
- **Exit** stops an active runtime, waits for its thread to finish cleanup, removes the icon and closes the server. A startup/cleanup error keeps the app open and displays the failure; it does not force-kill the process. The web Exit button retains its existing stop-first behavior.

The shell interface is `Shell_NotifyIconW` with a custom in-memory T icon, a hidden top-level window, and a blocking Windows message-loop thread. The hidden window receives `TaskbarCreated` to re-add the icon after Explorer restarts; that recovery path is implemented but has not been deliberately tested by restarting Explorer. No timer/polling loop, added dependency, separate helper process, new recurring log or driver installer is introduced. One additional idle thread services the tray. Menu commands reuse the token-authenticated loopback API; input processing remains on its own thread. No global keyboard/mouse hook is involved.

This first increment intentionally excludes restart/reconnect, profile selection, live status tooltips and autostart. It does not reload/reinstall the kernel driver, fix the existing device/sleep recovery limitations, or guarantee cleanup after forced termination. Opening configuration uses your default browser; closing it leaves the process and tray running.

Implementation references: Microsoft's [notification-area structure](https://learn.microsoft.com/windows/win32/api/shellapi/ns-shellapi-notifyicondataw) and [notification-area guidance](https://learn.microsoft.com/windows/win32/shell/notification-area). The menu uses the documented foreground-window and trailing WM_NULL pattern.

## Verification — 2026-10-15

31 default software tests pass, plus an explicitly run interactive-Explorer lifecycle test. The latter created the real icon, verified its shell rectangle and window, dropped the tray owner, and verified both icon/window removal. Clippy with warnings denied and optimized compilation pass. The filesystem may report a non-code incremental-cache hard-link warning.

A native-only backend smoke check exercised the same API handlers used by the tray: bounded start, observed running, Stop, observed stopped, another bounded start, and `/api/tray/exit` while active. Both runtime logs report zero inputs, `owned=0` and filter-clear requested. Evidence: `artifacts/tray/lifecycle.json` and its referenced runtime logs. No user keypresses, profile changes, driver installation or kernel-driver reload occurred. Menu placement, mouse selection and default-browser opening still need direct user confirmation; the automated checks do not claim those visual interactions were exercised.

To repeat the icon resource test on an interactive Windows desktop: `cargo test --offline tray::tests::explorer_icon_lifecycle -- --ignored --nocapture`. It does not activate interception.
