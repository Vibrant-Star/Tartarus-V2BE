# Keyboard runtime prototype

This is an incremental runtime, not the finished Synapse replacement. The Interception dependency must already be installed. It now selects one V2 MI_00 keyboard slot and one MI_02 mouse slot, revalidates IDs before processing/delivery, and rejects ambiguous selection. No global input hook is used. See the [wheel increment](wheel-runtime.md) for mouse-input support and its hardware results.

## Commands

Build with `cargo build --release`. Run from PowerShell, replacing paths as appropriate:

```powershell
& .\target\release\tartarus-v2be.exe run 'D:\Apps\Tartarus V2BE\vendor\interception\v1.0.1\Interception\library\x64\interception.dll' 'D:\Apps\Tartarus V2BE\config.example.json' 'D:\Apps\Tartarus V2BE\artifacts\keyboard.log' 60
```

The final argument limits the run to 1–300 seconds. Omitting it allows indefinite execution, but reliability validation is incomplete, so bounded runs remain recommended. Release all target and output keys before starting. No autostart entry or persistent process is created by the test harness. A runtime log must be a new file.

Stop a running instance with `tartarus-v2be stop`, using the same executable build and Windows session. This signals a named Windows event; it does not kill the process. Ctrl+C is also handled. A single runtime is allowed per Windows session. No console-free tray shell is implemented yet.

`scripts/Test-KeyboardRuntime.ps1` runs a bounded visible hardware check, applies a deliberately invalid configuration, switches a valid configuration live, stops the runtime at 60 seconds and keeps observing briefly to test native input restoration. Its local artifacts are timestamped and excluded from source control.

## Configuration

See `config.example.json`. Schema version 1 supports:

- `key`: one key name, such as `B`.
- `chord`: 1–8 distinct key names in press order. Releases occur in reverse order.
- `disabled`: suppress this control without replacement.
- `passthrough`, or an omitted mapping: retain its native keyboard action.

Controls `01`–`20` use the sequential capture's provisional physical labels. `thumb` names the captured native Left Alt signature. D-pad cardinal names use the observed extended arrow signatures. Do not treat the numbered Space control and a second Space-emitting thumb control as independent without additional evidence. `wheel_up`, `wheel_down` and `wheel_click` are also supported. Separate diagonal semantic actions and consumer-control collections are not handled yet.

Named outputs include A–Z, common modifiers, Space/Tab/Enter/Escape/Backspace and arrows. `scan:0030` and `scan:E0:48` provide explicit ordinary/extended Windows scan codes for other keyboard keys. Values are restricted to 01–7F. These are scan codes, not USB usages or virtual-key numbers. E1 output sequences are not implemented.

The runtime watches the configuration directory through Windows change notifications. Candidate files are capped at 64 KiB and parsed/validated completely before applying. Invalid versions, controls, actions, fields, key names or chords preserve the last valid mappings. Existing held outputs are released on a valid change, and physical controls held across the change are quarantined until release. The JSON on disk persists across program starts.

The input engine resolves each press once, reference-counts shared outputs, and unwinds the resolved action on release. Repeat downs do not add owners. A repeated chord repeats only its final non-modifier output. The delivery ledger separately tracks successful output downs for best-effort error cleanup.

`check-config <JSON-path>` validates a file without opening the device or enabling a filter. Duplicate control names are rejected, and a Windows UTF-8 BOM is accepted. Startup refuses takeover while a keyboard key is observed held, to reduce the risk of consuming a native release whose down preceded activation.

## Hardware verification — 2026-10-15

First runtime test (`artifacts/interception/runtime-20261015-202351/`) rejected deliberately invalid JSON, kept its prior configuration, applied revision 2 without restart and stopped with `owned=0`. Captured B repeats during holds were expected typematic events. The user missed the C stage, so that capture alone was not counted as evidence of C delivery. Native Tartarus 2 events were recorded after automatic stop.

The repeat (`artifacts/interception/reload-20261015-202727/`) started with B and applied C in the same process, revision 2 at roughly 265 ms. It recorded four C press/release pairs in both target Raw Input and ordinary Windows messages. No native target 1 appeared during that active stage. The stop command ended the runtime before its safety deadline with `owned=0`; the observer then recorded one native target 1 press/release pair in both input paths. There were zero source lookup errors and no runtime error output. This verifies live remapping and graceful stop/restoration for that sample.

Twelve software tests cover validation, control identity, shared outputs, chord overlap, repeats, reload quarantine and cleanup semantics. Formatting, Clippy and the optimized build pass; the current prototype executable is about 384 KiB. Chords and broad rollover still need dedicated physical testing. No automatic process remains active after these bounded tests.

## Waiting and limitations

Input uses the Interception blocking wait with a 250-ms timeout because its public API does not expose a combined wait with configuration and stop events. Input events wake it immediately; configuration/stop handling can take up to roughly 250 ms while idle. The runtime does not repeatedly read the file while idle. Hardware identity is checked after waits and before delivery; overhead has not been benchmarked.

The filter query still reports zero even when functional interception is observed. Cleanup requests are logged, but readback alone is not proof of restoration. The backend exits on an observable identity change; automatic reconnect, sleep/session-lock handling, identical-unit container selection, forced-termination cleanup, and cleanup when device delivery fails remain unfinished. It must not silently continue remapping through another backend.

Ownership accounting applies to this keypad's outputs. Overlapping same-key/modifier ownership with another physical keyboard remains an explicit hardware acceptance gate. Do not infer that reference counting alone resolves Windows' shared keyboard state. Thumb modifier, momentary and cycle modes are now implemented; see [layers.md](layers.md) for scoped validation. The [local web editor](web-ui.md) is now implemented. Virtual mouse output actions, media/macros, RGB, tray, autostart and packaging are subsequent phases.

Recovery follow-up (2026-10-16): automatic device/power session rebuilding and release-only fallback cleanup are now implemented; see [recovery behavior, validation and limits](recovery.md). Earlier disconnect behavior described above records the preceding increment. Hardware sleep/session-lock and broader privilege/consumer gates remain separate.
