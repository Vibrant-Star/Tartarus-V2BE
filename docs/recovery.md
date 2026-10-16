# Device and sleep recovery

The requested runtime now keeps its stop handle and configuration watcher across capture sessions. A native CM_Register_Notification HID callback signals only paths whose exact VID/PID tokens identify Tartarus V2 (1532:022B). PowerRegisterSuspendResumeNotification handles suspend and both automatic/user resume notifications. Neither registration requires a new process, hidden polling window or kernel driver change. Callbacks signal an event and generation counter; device callbacks do no driver I/O.

A device/power generation change invalidates the session before further input processing. The runtime releases recorded successfully delivered outputs, clears capture filters, destroys the old Interception context, resets layers/physical holds/wheel fragments and discovers both interfaces again. Old numeric slots are never reused without enumeration. Recovery refuses zero or ambiguous targets, missing interfaces, and held global keys/buttons. Current global key-state gating is not proof of a device's physical neutral report, and the race between neutral checking and capture remains a hardware validation concern.

While unavailable, the UI exposes Waiting to reconnect and Stop remains available in the UI/tray. The runtime uses a stop/device-event wait with one-second retries during the first ten failed attempts, then a 30-second fallback interval; native PnP events wake it immediately. Waiting for held keys checks at 250 ms. Suspension waits for a resume/stop event rather than reconnecting repeatedly. Normal connected capture retains the existing blocking 250-ms backend wait. Unchanged wait errors are not repeatedly logged; transitions are logged. No new per-key disk logging was added.

On known device/power loss, cleanup uses only the reverse ledger of outputs this process successfully pressed. A release-only SendInput fallback bypasses the unavailable device slot; it never sends a new press, motion, wheel impulse or blanket release of every modifier. Normal shutdown first uses the identified backend and falls back if that fails. Wheel impulses have no cleanup state. Cleanup failure stops automatic recovery and reports an error. This does not solve Windows' shared state with another real keyboard/mouse owning the same output, secure desktop/UIPI restrictions, raw-input-only consumer semantics or forced process termination.

The suspend callback allows up to one second for cleanup acknowledgement. Windows may suspend before worker cleanup finishes; generation changes are retained and cleanup/rebuild occurs on resume before further mapped processing. This is best-effort cleanup, not a guarantee against sudden power loss. One manual sleep/wake cycle has now passed; see the hardware-check result below.

Sources: [PnP registration](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_register_notification), [event layout](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/ns-cfgmgr32-cm_notify_event_data), [power registration](https://learn.microsoft.com/en-us/windows/win32/api/powerbase/nf-powerbase-powerregistersuspendresumenotification), [suspend deadline](https://learn.microsoft.com/windows/win32/power/pbt-apmsuspend), and [SendInput privilege limits](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput).

## Validation

43 software tests pass and one Explorer lifecycle test remains opt-in. New tests verify exact V2/Pro discrimination, notification ABI sizes, real Windows registration/unregistration, synthetic target-only device callback wakeups and resume-state changes, and release-only keyboard/mouse encodings. No test invokes system sleep or injects input through the cleanup API. Formatting, Clippy with warnings denied, browser JavaScript syntax and optimized build pass.

The /recovery-test page is an explicit in-memory observation tool for left mouse output and B. Its counters include other physical-device activity and cannot independently attribute source or establish global Windows state. The test guard saves/restores the original active JSON and stops after 240 seconds, or earlier when Stop is requested. Library profiles are untouched.

## Cable hardware check

The user confirmed B stopped when unplugging while holding key 02, and both key 01 left-click and key 02 B recovered after reconnecting without restarting the app. The runtime log records two disconnect/reset/wait/fresh-capture cycles during the bounded test, followed by a clean stop. The browser ended with 8 B downs, 8 ups, 134 repeats and held=false; counters cannot independently identify the source. The original active configuration was restored and verified byte-for-byte. Evidence: artifacts/recovery-test. Sleep/resume is still pending at this stage.

## Sleep/wake hardware check

The user performed a real Windows sleep/wake cycle with the cable connected, and confirmed both mappings recovered without restarting the application. Their initial answer saying they did not recover was immediately corrected to "They did recover"; the corrected answer is the result recorded here.

The log confirms a device/power session reset, Windows-suspended waiting state, a fresh active capture session, and a clean stop with 20 input packets, owned=0 and filter-clear requested. Stop was confirmed without a runtime error, and the original active JSON was verified restored byte-for-byte. Evidence is under artifacts/sleep-test.

This establishes one manual sleep/wake cycle in the current environment, alongside the cable check above. Repeated stress, hibernation, secure desktop, forced termination, higher-privilege applications, and holding mapped outputs through suspend remain broader acceptance checks. Shared ownership with separate physical keyboards/mice remains unverified as previously documented.
