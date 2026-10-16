# Incremental delivery and release gates

No complete phase acceptance has been claimed yet. Working increments now include passive control captures, a bounded verified single-key suppression test, and an experimental keyboard runtime with hardware-verified live reload and stop/restoration. See [runtime results](keyboard-runtime.md), [layer results](layers.md), and the [local editor](web-ui.md). The editor is implemented and browser-tested; a bounded key-01 test verified UI-driven B-to-C live reload, browser-tab independence and native restoration. Broader phase acceptance remains open. Each phase must produce an executable test and recorded results before advancing. Automated engine tests cannot substitute for Windows/hardware validation.

| Phase | Deliverable | Exit gate |
| --- | --- | --- |
| 1 | Rust read-only inventory and event diagnostics | Enumerate local HID capabilities and parents; label every physical control; save target-only traces; verify unrelated device exclusion |
| 2 | Selective suppression and one-key remapping | Approved backend; original event absent in both legacy and Raw Input consumers; replacement appears once; other keyboards/mice unchanged |
| 3 | Layers, output ownership and actions | Chords, media, mouse and macros pass overlap/cancellation tests; all thumb modes work; shared physical Alt passes |
| 4 | Versioned JSON and visual localhost UI | Valid edits apply immediately; invalid edits preserve active state; create/duplicate/import/export profiles; browser exit leaves runtime active |
| 5 | V2 lighting implementation | Audit and test every command; requested effects, brightness and off verified; restart/reconnect restores configuration |
| 6 | Tray, startup and packaging | No console on normal launch; per-user startup round trip; documented dependency install/remove and license inventory |
| 7 | Reliability and resource validation | Hardware stress, fault recovery, latency and idle-resource measurements meet recorded budgets |

## Phase 1 capture procedure

Use SetupAPI and HidD/HidP capabilities plus Raw Input source names. Windows preparsed data is not a byte-for-byte USB report descriptor; keep these artifacts distinguished. Where protected system collections prevent raw report reads, record that limit and use Raw Input keyboard/mouse transitions. Do not replace keyboard drivers with WinUSB just to capture reports.

1. Record OS, firmware if readable without writes, collection paths/usages, parent/container relationships and active driver stack. Exclude unrelated device data from saved exports.
2. Record press/release of every labeled key and thumb control individually. Distinguish keyboard repeats from transitions.
3. Record cardinal D-pad directions, every diagonal, clockwise/counterclockwise rolls and neutral transitions.
4. Record wheel up/down at different speeds, middle-click and click-plus-scroll.
5. Record overlapping keys in increasing counts, modifiers and cross-interface combinations. Identify rollover/error reports and any indistinguishable controls.
6. Repeat after unplug/replug and sleep/resume. Determine whether current state is available or neutral synchronization requires a release/repress sequence.

Keep a manifest of expected controls with observed event source, scan code/usage/report ID, press/release or impulse semantics and evidence filename. Unknown entries remain unsupported rather than receiving guessed defaults.

## Phase 2 backend experiment

Before installation, document exact package/version, source availability/license, signing, administrator/reboot requirements, all-device filter impact, rollback, and Windows 11 security compatibility. Obtain approval for any kernel installation/modification. Do not disable Secure Boot or Memory Integrity as a workaround.

Test every active keyboard and mouse collection. Demonstrate source-to-container identity without inferring origin from event timing. Include two identical V2 units if available. Test backend process death, driver availability loss, reconnect and changed numeric device IDs. If the backend cannot isolate the target, remapping stays unavailable.

## Required regression matrix

- Real keyboard holds Alt -> V2 mapped Alt press/release -> real Tab; reverse ownership order and release order too. Repeat for Ctrl/Shift/Win and mouse buttons.
- Two mapped controls share one output; release either first. Chords overlap macros and normal typing.
- Layer/profile changes while controls are held; momentary nesting, cycling repeats and D-pad diagonal transitions.
- Disconnect/suspend/exit mid-chord and mid-macro; forced termination separately from orderly shutdown; fresh startup after a crash.
- Fast wheel pulses, simultaneous middle-click, target and real mouse activity; native input never duplicated.
- Invalid/truncated/imported config, interrupted save, oversized HTTP body, foreign Origin and Host, occupied port and second runtime instance.
- Standard/elevated target apps, legacy input, Raw Input and representative games. Document unsupported paths rather than claiming universal delivery.
- Driver present/absent, Synapse absent, and existing Razer filter present; security features enabled. Record exact environment for each result.

Measure idle CPU/RAM, input latency distribution under load, reconnect time, dropped events, queue depth and held-state ledger after stress. Choose numerical release budgets from a measured baseline. Do not invent performance claims before an executable exists.
