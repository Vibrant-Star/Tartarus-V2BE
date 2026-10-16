# Local configuration editor

The current editor uses [responsive section navigation](ui-layout.md), including a horizontal macro editor. Earlier validation notes below describe the initial editor; macros, mouse/media outputs, RGB and startup options have since been implemented and have their own linked documentation.

Run `cargo build --release --offline --target-dir target/ui`, then `powershell -ExecutionPolicy Bypass -File scripts/Open-UI.ps1`. This invocation allows the local launcher script for this process only; it does not change the machine execution policy. The launcher starts the executable hidden and opens the default browser. The launcher itself is a script; a native standalone GUI launcher remains a packaging milestone.

The editor listens exclusively at **http://127.0.0.1:17832/**. It uses embedded HTML/CSS/JavaScript with no framework, CDN, frontend build or Electron. ZIP/XML import uses lightweight in-process parser dependencies. Opening it does not load the Interception DLL or activate filters. The existing DLL path must be present; no installer or driver changes run here.

On first launch, `data/config.json` contains only native assignments. `data/profiles/` holds JSON library profiles; `data/logs/` holds runtime and launcher logs. These private/generated files are ignored. The launcher reopens a listener already using the fixed port; if another application owns it, exit that application or launch the executable directly to see the bind error. It never kills an existing listener.

## Editing

- Click any numbered control, D-pad direction, wheel action, or thumb. Select native input, a keyboard key, a combination, or disabled. **Update assignment** updates the draft; **Apply changes** validates and saves the whole draft. An active runtime applies the file without restarting, normally within its existing 250 ms idle control interval. The header distinguishes saved/pending and runtime-confirmed applied settings.
- Select, add, or rename independent layers (up to three). The editing layer and the currently active runtime layer are separate labels. Thumb modes are per-layer assignment, modifier, hold-for-layer, and cycle through all configured layers. Selecting a shared thumb mode clears its per-layer assignments in the draft; reloading saved settings undoes unapplied edits.
- The combination recorder uses browser keyboard events while recording. It supports up to eight keys, times out after ten seconds, and cancels on Escape or focus loss. OS-reserved shortcuts may not reach it. This is a configuration recorder, not an interception backend. Scan-code names can be entered manually for additional keys.
- New/duplicate profiles are drafts. Save to library persists a named copy independently of the active configuration. Selecting a library profile previews it; Apply activates it. JSON import is validated before becoming a draft; export includes the current validated draft. [Synapse 3 import](synapse-import.md) now provides preview and passive source preservation.

The visual layout is a schematic using the existing provisional control labels. D-pad labels describe captured arrow signals; compass orientation is not calibrated. The numbered Space control appears as key 20 at the far lower right; the wheel is beside key 19 and the small HyperShift control is above the D-pad. It is not duplicated as another physical button.

## Runtime and lifecycle

**Start remapping** explicitly starts the existing device-specific backend in a separate thread. The server keeps input processing independent of browser requests. Startup errors (including held keys, missing target interfaces or missing driver access) appear in the header, with the full runtime result in the log. Device/driver status is confirmed only while this managed runtime has successfully connected; while stopped, it explicitly says not checked. A separately launched command-line runtime is not managed by this editor; attempting to start a second runtime fails.

Closing the browser does not stop the server or input runtime. **Stop** requests graceful releases and filter cleanup; wait for stopped status. **Exit app** closes the server only after its managed runtime has stopped. The [tray Exit action](tray.md) also requests graceful stop when needed. Reopening the browser reconnects to the same server. Nothing is installed into autostart. Existing disconnect/sleep/crash reliability limitations still apply; the editor does not add a fallback backend.

## Local API boundaries

Requests require the exact loopback Host. API reads and writes require a per-launch 256-bit Windows-generated token embedded in the same-origin page; foreign Origin values are rejected. No CORS permission is sent. Responses use a restrictive content policy, no-store caching, and deny framing. JSON posts, 8 KiB headers, 150 kB bodies and two-second connection timeouts bound this small HTTP subset. Transfer encoding and duplicate headers are rejected. Connections are served sequentially; input runs separately. This is a local utility, not a general HTTP server or protection against malicious software running as the same Windows user.

Active configuration saves validate the complete schema, compare the editor's original file text to detect stale edits, write/sync a same-directory temporary file, and replace the target through Windows. This protects against partial writes; the comparison is not a cross-process file lock. Invalid or stale saves keep the existing configuration. Runtime confirmation comes from its successful configuration processing, rather than assuming a successful HTTP save means inputs were already changed.

## Validation

Software tests cover request limits/ambiguity, loopback/token/origin checks, invalid and stale writes, and successful replacement, in addition to the existing input/configuration tests. Browser checks cover key/chord editing, persistence, layer selection, profile-library saving, and combination recording. Hardware remapping was left stopped during the initial editor checks; the subsequent bounded hardware result is recorded below. Macros, mouse/media output actions, RGB and autostart are not exposed as working controls. [Basic tray controls](tray.md) are now implemented.

2026-10-15 evidence: `artifacts/ui-preview/` contains the isolated browser-check configuration/library, `api-checks.json`, and `editor.jpg`. The saved chord and library profile survived server restart. Live API checks returned 403 for missing token and foreign Origin, 400 for invalid configuration and 409 for a stale edit; disk configuration stayed unchanged. The Exit endpoint terminated the idle test server. The launcher was then run twice against the working configuration: one loopback listener remained, with native defaults and remapping stopped. The optimized executable is 683,520 bytes. All 26 tests, Clippy with warnings denied, JavaScript syntax checking and optimized build passed (the filesystem emitted a non-code incremental-cache hard-link warning).

## Bounded UI-to-hardware result — 2026-10-15

Evidence: `artifacts/interception/ui-20261015-213316/`. Through the visual editor, key 01 was assigned B and Start was selected with the new **60-second test** duration. The runtime confirmed active/applied. The observer recorded six B press/release pairs. Editing and applying C through the same UI produced revision 2 at 26.470 seconds without restarting; five C pairs followed. The configuration tab was closed during the C stage and reopened after the runtime's automatic deadline. The original runtime log's last write was approximately 60.035 seconds after its start; shutdown reported 22 inputs, two revisions, `owned=0` and filter-clear requested. Three native 1 pairs followed after shutdown.

Raw Input and ordinary Windows message counts matched: B 6/6, C 5/5, native 1 3/3. No B downs followed the revision boundary, no C downs preceded it, and no native 1 downs occurred during the active interval. There were no unmatched releases, held keys at completion, observer source lookup errors or observer stderr output. This verifies a basic UI edit/start/live-reload path, browser-tab independence, bounded shutdown and native restoration for this sample. It does not establish every action/control or the remaining reconnect/shared-device reliability gates. The original native-only working configuration was restored and the editor left open with remapping stopped.

The runtime duration selector defaults to **Until stopped** and also offers a **60-second test**. Test deadlines are enforced by the input runtime, independently of browser timers or an open page.
