# Wheel and middle-click increment

The runtime now opens one verified V2 MI_00 keyboard slot and one verified V2 MI_02 mouse slot through the same Interception context. Both must be identified unambiguously; there is no keyboard-only fallback if the required mouse path is unavailable. Other keyboard/mouse slots are not selected by either filter predicate. IDs are revalidated before delivery. Full container-based multi-unit selection remains unfinished.

JSON controls `wheel_up`, `wheel_down` and `wheel_click` support the existing key/chord/disabled actions. Omitted or passthrough mappings retain native mouse behavior. See `config.wheel.example.json` for an A/B/C test profile. This increment maps mouse controls to keyboard actions; mapping keyboard controls to virtual mouse buttons or wheel actions remains subsequent work.

Vertical wheel increments are accumulated in signed 120-unit steps. Larger deltas generate multiple taps, partial deltas accumulate, and opposing partial deltas cancel. Each complete step invokes a press/release through the shared output-ownership engine. Accumulation is reset on configuration changes. Actual hardware captures so far used increments of +120/-120; partial increments are software-tested behavior.

Middle-click resolves its mapped/native route on press and retains it until release. This prevents a configuration change from consuming the release of an earlier native press or forwarding the release of a suppressed mapped press. Mapped holds participate in the same ownership and reload-quarantine mechanism as keypad keys.

Combined packets are split field-by-field: only mapped middle-button/vertical-wheel flags are consumed. Other button flags, motion, mouse attributes, horizontal scrolling and extra information remain in the residual native packet. The mouse C layout is independently represented and checked as 20 bytes. The implementation follows the public Interception API; no additional driver was installed.

`scripts/Test-Wheel.ps1` runs a bounded visible A/B/C test, compares regular-mouse scrolling/middle-click, and then stops the runtime for a native restoration capture. The observer logs only the requested non-target mouse wheel/middle bits under an anonymous `other-mouse` label, omitting pointer motion and unrelated buttons.

## Validation

Eighteen software tests pass, including combined report preservation, multiple/partial/reversed scroll deltas, middle-button routes across configuration changes, and shared keyboard output ownership. Wheel impulses repeat their final non-modifier key if another action already holds that output, without releasing the existing owner. This same-output edge case is software-tested; the hardware profile used distinct A/B/C outputs. Clippy and the optimized build pass.

## Hardware results — 2026-10-15

The first capture (`artifacts/interception/wheel-20261015-204613/`) recorded 10 A taps, 9 B taps and 2 C press/release pairs. Raw Input and ordinary Windows messages agreed. No original Tartarus vertical wheel or middle-button events escaped during mapping; no mapped key remained held and no unmatched releases were found. The regular mouse retained both scroll directions and middle-click. After stopping, both native Tartarus scroll directions and middle-click were observed again, with 24 positive and 22 negative wheel events plus two middle-click pairs.

The user confirmed the combined gesture was missed in the first capture, so it was not counted as a combined-input pass. A focused repeat (`artifacts/interception/wheel-combined-20261015-204923/`) recorded 33 A taps, 29 B taps and 3 C press/release pairs. Of the 62 scroll taps, 41 occurred while C was held. Raw Input and ordinary message counts again matched, with zero original-input leakage, zero unmatched releases, zero source lookup errors and no held mapped keys at completion. The runtime logged 68 received events and `owned=0` at shutdown. The test does not establish whether click and scroll shared a single hardware report; combined report fields are separately covered by software tests.

Both tests were bounded and have stopped. These results validate vertical scrolling and middle-click-to-keyboard remapping on the connected V2, including simultaneous click/scroll. No persistent profile or autostart was enabled.

Protocol sources:

- https://github.com/oblitum/Interception/blob/v1.0.1/library/interception.h
- https://github.com/oblitum/Interception/blob/v1.0.1/library/interception.c

The known filter-readback discrepancy, lack of automatic reconnection/sleep handling, and cross-keyboard shared-modifier uncertainty still apply. This increment is not a finished driver replacement.
