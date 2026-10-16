# Passive hardware diagnostics

## Post-reboot result, 2026-10-15

Both Interception kernel services report RUNNING. The x64 probe succeeds outside the restricted agent process and identifies candidate V2 keyboard slots 1 (MI_00), 2 (MI_01 Col01), and mouse slot 11 (MI_02). These numbers are observations for this boot, not persistent identities. The sandboxed invocation failed due to restricted access; that was not a driver-load failure.

Raw Input inventory also finds additional MI_01 HID collections. Their presence does not prove which controls use them or that Interception covers their output. Two-second passive smoke test completed successfully with zero events and zero source lookup errors. No remapping or capture filters were enabled.

PnP enumeration outside the sandbox subsequently succeeded. All locally present V2 interfaces share a single container and lead back to the same USB composite parent; the local evidence is saved in `artifacts/interception/target-pnp-after-reboot.json`. The runtime does not yet enforce container selection.

The first 60-second capture recorded 60 target events and zero source lookup errors. Observed keyboard scan codes on MI_00 were extended 48/4B/50/4D (arrow virtual keys), 38 (Alt), and 39 (Space). MI_02 reported wheel deltas +120/-120 and middle-button down/up flags 0010/0020. The user confirmed numbered keys were not pressed during this first capture, so their absence is not a hardware failure. Physical D-pad/numbered/thumb associations still require explicit confirmation.

The second 60-second capture, requested for numbered keys in order, recorded 40 keyboard events: 20 distinct signatures, each with one press and one release, no repeats, no unmatched releases and no held keys at completion. Every event used MI_00. The observed order is saved in `research/observed-numbered-controls.json`; labels remain provisional until the physical order is confirmed. Native virtual keys were: 1, 2, 3, 4, 5, Tab, Q, W, E, R, Caps Lock, A, S, D, F, Shift, Z, X, C, Space. The numbered capture's Space signature overlaps the first capture's Space; do not invent a second independently distinguishable Space control from these traces.

These passive captures establish individual transitions, not simultaneous rollover, D-pad diagonals or correct shared-modifier behavior. A subsequent [bounded 1-to-B trial](single-key-trial.md) verified single-key suppression/replacement and the unrelated-keyboard comparison using a visible observer. Persistent remapping remains disabled.

## Capture command

```powershell
& .\target\release\tartarus-v2be.exe capture 60 'D:\Apps\Tartarus V2BE\artifacts\interception\my-new-trace.tsv'
```

The output file must not already exist. Duration is bounded to 1–300 seconds. The executable uses a message-only window and blocking Windows message loop with a timer; no idle polling. Target keyboard, mouse and consumer-control events are observed through Raw Input. Original input is not suppressed, so focus a harmless window while exercising the device. It is deliberately a console diagnostic, not the eventual background runtime.

After capture, `scripts\Summarize-Trace.ps1 -Path <trace-path>` reports key signatures in first-observed order, repeats, unmatched releases and held keys at the end. This is a diagnostic summary, not a physical mapping generator.

Trace rows include elapsed microseconds, scan code/flags/virtual key or mouse button flags/wheel delta, and device path. HID rows retain payload bytes rather than guessing their meaning. Timestamps describe application receipt, not USB latency. Unrelated device identities and events are omitted from saved traces. Candidate identity matches exact V2 VID/PID tokens; identical V2 units are not yet disambiguated. Unknown source lookups are counted rather than assigned to the Tartarus.

Test the numbered keys sequentially with release between keys, then D-pad cardinal and diagonal directions, wheel in both directions, wheel click and thumb controls. A trace alone cannot establish physical labeling without a known order or user annotations. Follow-up captures must cover rollover and simultaneous inputs. No claims of full control coverage or selective suppression follow from successful inventory.

## API references

- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerrawinputdevices
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getrawinputdata
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-rawkeyboard
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-rawmouse

The observer uses INPUTSINK and DEVNOTIFY, without NOLEGACY. Five tests currently cover source identity, hardware-ID decoding, Windows x64 ABI layouts and trial mapping ownership/passthrough. These are software checks; physical mappings still require traces and confirmation.
