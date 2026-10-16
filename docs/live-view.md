# Live physical-input view

Enable **Live input view** above the keypad while remapping is running. Physical holds use a blue highlight distinct from the green selected editor control. Short taps and wheel notches flash for 200 ms. Middle-click and both wheel directions are observed even if they are native or disabled. There is no global keyboard hook or Raw Input fallback; only events already captured from the selected Tartarus backend feed this view. When remapping is stopped, the UI says to start it rather than pretending to observe the device.

**Follow active layer** changes the editing layer to match the runtime's layer. Turn it off to inspect another layer. Following pauses while an unapplied draft or an assignment/name field is being edited, so it cannot overwrite pending form input. Live highlights still operate during editing. The active runtime layer is shown separately. The last-press line describes the action configured in the applied runtime layer, including native/disabled actions and thumb mode; it is not an acknowledgement that Windows delivered each output to a target application. A quarantined physical hold remains visibly held even though the mapping engine may have released its output on a layer/profile transition.

The view clears on normal stop or observed device/backend failure. Browser connection loss clears immediately; hidden tabs abort their stream and reconnect with the current state when visible. Closing the UI leaves the runtime and tray running. Forced process termination and the existing sleep/reconnect limitations remain separate gates.

## Performance and boundaries

`live.rs` tracks at most the 28 known control IDs, held state and monotonic press counters. Counters let a slow viewer flash a short tap even if its press/release snapshots were coalesced. The same bounded snapshot includes layer names and last configured action. Repeats do not count as new physical presses. Passive mouse normalization receives a copy of the packet; it does not change interception/injection routing.

An opt-in authenticated `GET /api/live` fetch receives server-sent-event framing. The token is a header, not a query parameter; existing exact Host/Origin checks apply. [Fetch/SSE framing](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events) is consumed with a streaming reader without new libraries.

There are at most four viewer threads. A viewer waits on a condition variable and receives the latest snapshot, with a 15-second idle heartbeat and two-second socket write timeout. Slow clients do not create event queues or perform network writes on the input worker. Disconnect detection may retain a viewer slot for up to a heartbeat/write interval; rapid repeated toggling can temporarily hit the four-view limit. Browser retries are bounded in frequency (three seconds). The existing browser status check remains separate at 1.5 seconds while visible. No live input snapshots are logged to disk by the application. Existing diagnostic runtime logging is unchanged.

With no viewers there is no live serialization/network traffic; bounded physical state is still retained so enabling the view mid-hold starts with the correct state. No process or dependency was added. The tray remains in the same process.

## Appearance

The page uses its bundled Inter font and explicit dark colors. The user confirmed that Zen now displays the correct font and dark theme, and identified an enabled browser Boost as the source of the earlier appearance override. No browser settings were changed by the application.

## Validation — 2026-10-16

34 software tests pass; one interactive Explorer lifecycle test remains opt-in and had passed in the preceding tray increment. New tests cover repeat/held state, native middle-click and multiple wheel impulses, known/unknown control identity, per-layer labels, and bounded/coalesced viewer slots. Clippy with warnings denied, JavaScript syntax and optimized build pass (the filesystem may emit a non-code hard-link cache warning).

A bounded 60-second hardware sample used key 01 A/B and momentary Shifted, with disabled wheel outputs. The observer collected 31 in-memory snapshots: seven key-01 presses, seven thumb presses, seven Base→Shifted→Base round trips, observed held 01/thumb, and a final stopped frame with no held controls. During the test the browser DOM showed Shifted selected and the thumb's physical-held class, with no browser warnings/errors. The runtime stopped with 28 input packets, `owned=0` and filter-clear requested. No wheel input or shifted key-01 press was captured in this sample; those remain hardware follow-up checks, despite software coverage. Evidence is `artifacts/live-view/summary.json` and `during-test.jpg`. The observer saves a final summary only; it is an explicit test tool, not runtime logging.

The original active configuration was restored after the sample. The imported example library profile remains untouched.

## Bundled font follow-up

At the user's suggestion, Inter Variable is now embedded in the executable and served from the hash-versioned localhost URL `/fonts/inter-693b77d4.woff2`. No font provider is contacted at runtime. Its 352,240-byte upstream file, commit/SHA-256 provenance and SIL OFL 1.1 license are in `web/fonts/`; the license is also served at `/fonts/LICENSE.txt`. Source: [Inter's official font files](https://github.com/rsms/inter/tree/master/docs/font-files). The font response is binary-safe, uses `font/woff2`, is allowed by a same-origin-only `font-src` policy, and is cached under its content-versioned filename. No system font is installed.

The browser loads it as the private family `Tartarus UI`; no `local()` alias can substitute a different installed font. The in-app browser reports bundled font loaded true, background `rgb(16,19,21)`, `dark only` and forced-colors false; its console has no font/CSP warnings. Downloaded response bytes match the bundled SHA-256. The user confirmed correct font and dark theme in Zen after this update. The expanded Appearance details panel reports this build, font-load state and computed colors without changing browser settings. The final executable is 1,469,952 bytes.

Four authenticated streams were opened concurrently; a fifth was rejected with HTTP 429. The regular status request still completed in 40 ms with all four open. An unauthenticated stream request returned HTTP 403. These are local functional checks, not a performance benchmark.

Recovery follow-up: device/power resets now clear the live frame while the runtime waits to reconnect; fresh sessions reset the physical counters and active layer. See [recovery](recovery.md) for current validation and limits.
