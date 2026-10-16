# Thumb modes and independent layers

The runtime supports up to three total layers. Existing flat schema-version-1 JSON remains compatible. Root `mappings` is layer 0; optional `layers` entries add layers 1 and 2, each with a unique name and independent mappings. An omitted mapping retains the control's native action rather than inheriting layer 0. Keyboard, D-pad and wheel controls share the same layer selection.

See `config.layers.example.json`. Root `thumb` selects one behavior:

```json
{ "mode": "modifier", "key": "LeftAlt" }
```

Modifier mode sends the configured standard modifier, defaulting to LeftAlt when `key` is omitted. With no root `thumb` field, legacy thumb mappings or native Alt passthrough still apply. Do not specify both root thumb mode and per-layer `thumb` mappings.

```json
{ "mode": "momentary", "layer": 1 }
```

Momentary mode consumes the captured thumb Alt events, switches while held and returns to the previous layer on release. Holding the thumb does not create repeated switches.

```json
{ "mode": "cycle", "layers": [0, 1, 2] }
```

Cycle mode advances once per physical press, ignores repeat downs and remains on the selected layer after release. Use `[0,1]` for two layers. Cycles require 2–3 distinct configured indices and start with 0.

## Transition policy

A layer change releases the old owned outputs and quarantines controls still physically held until their releases. A fresh press uses the new layer. This prevents a held key from unexpectedly activating a different action. The thumb trigger itself stays tracked until release, so momentary layers return correctly. A profile reload resets layer 0, releases owned outputs and suppresses an old held thumb's trailing release if its mode changes. Wheel partial-delta accumulation is reset at layer transitions and profile reloads; middle-click retains its press-time native/mapped route until release.

Layer routing runs on the device event path, without waiting for a file watcher or restarting the runtime. Changes are logged with old/new layer and elapsed time. This establishes ordering, not a measured latency guarantee. The thumb signature is the V2's captured unextended Left Alt scan code; the numbered Space control is independent. No hook or filter is applied to another keyboard's Alt.

## Validation

Twenty-three software tests cover repeat suppression, 3-layer cycle order, old-key release and fresh-press behavior, profile reload while the thumb is held across passthrough/layer modes, default modifier validation and configuration conflicts. Existing keyboard/mouse ownership tests also pass. Hardware results are appended after the bounded captures.

`scripts/Test-Layers.ps1 -Mode momentary`, `-Mode cycle`, and `-Mode modifier` prepare separate bounded visible experiments. The test profile maps key 01 and positive wheel steps to A/B/C on layers 0/1/2. The window title remains a last-key display, not a current-layer or held-key indicator.

Cross-keyboard same-modifier ownership, sleep/reconnection and forced-termination cleanup remain separate reliability gates. Passing a layer test does not close those gates or establish complete project acceptance.

## Momentary hardware result — 2026-10-15

Evidence: `artifacts/interception/layers-momentary-20261015-205624/`. The runtime recorded eleven base-to-shifted and eleven shifted-to-base transitions. The observer saw 25 A press/release pairs on layer 0 and 17 B pairs on layer 1; no A downs occurred on layer 1 and no B downs occurred on layer 0. Raw Input and ordinary Windows message counts agreed. There were no target native key-01 or thumb-Alt events during remapping, no unmatched letter releases, no held letters at completion and zero source lookup errors. The runtime stopped with `owned=0`.

This sample verifies holding/releasing the captured thumb changes layers without native Alt leakage. The counts combine mapped keypad and wheel input; per-control timing is not a latency benchmark. The fresh-press policy across a held-key transition is software-tested and should not be inferred from aggregate pair counts alone.

## Initial cycle result — 2026-10-15

Evidence: `artifacts/interception/layers-cycle-20261015-205955/`. The observer associated all 16 A downs with layer 0, all 13 B downs with layer 1 and all 14 C downs with layer 2. There were no wrong-layer letters, native key-01/Alt leakage, unmatched releases or held mapped letters at completion. Every recorded transition followed 0→1→2→0.

The user nevertheless reported occasionally seeing A followed by C. Some forward transitions were less than 0.2 seconds apart, so a visible intermediate layer can be skipped if two presses are accepted before the next key-01 tap. The initial log recorded transitions, not raw thumb press/release edges, and cannot distinguish extra physical edges from intentional multiple presses. Added thumb edge/repeat logging for diagnosis; no debounce or press suppression was introduced on this evidence alone. The user reported existing contact issues and occasional double taps on this older device and accepted this behavior for now. This is consistent with extra forward steps, but the capture does not prove an electrical cause.

## Modifier hardware result — 2026-10-15

Evidence: `artifacts/interception/layers-modifier-20261015-210413/`. This stage intentionally configured the thumb as LeftAlt. The user confirmed Alt+Tab worked and subsequently confirmed the overlap check: hold regular-keyboard Alt, press/release the Tartarus thumb, then press Tab while regular-keyboard Alt remains held. No layer transitions were expected. The runtime processed 143 input packets and stopped with `owned=0`; the observer recorded zero source lookup errors.

Windows consumed shortcut Tab downs before the observer could record them. The trace cannot reconstruct every shortcut or independently establish the overlap check's timing within the bounded active interval. The overlap result is user-reported; broad cross-keyboard modifier reliability remains an acceptance gate. The recorder now labels the last observed key without calling Alt “native”; its title does not identify the event's origin or whether interception is active.