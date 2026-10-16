# Proposed architecture

Status: design, conditional on the interception feasibility gate. Prefer one Rust executable running in the signed-in user's session, with embedded HTML/CSS/TypeScript assets. A Windows Session 0 service is not the default input injector. No Electron or Synapse dependency.

## Modules

| Module | Responsibility |
| --- | --- |
| device | SetupAPI/HID inventory, collection capabilities, container association, connection generations, device notifications |
| diagnostics | Raw Input observation, timestamped target-only traces, physical-control labeling; never suppresses or injects |
| capture | Backend capability negotiation, exact target selection, native suppression and explicit unavailable state |
| mapping | Pure deterministic transition engine; no Windows calls |
| output | Owned key/button states, ordering, backend delivery acknowledgements, cleanup |
| lighting | Audited V2 commands and response handling; independent from input worker |
| config | Versioned JSON, strict validation, atomic persistence and runtime transactions |
| web | Embedded assets and authenticated localhost API |
| shell | Tray, single-instance coordination, per-user startup, session and power notifications |

## Input path and identity

device identity -> verified capture backend -> normalized physical transition -> mapping engine -> output ledger -> injection backend

Keep Raw Input diagnostics separate from remapping event delivery to avoid processing one event twice. Match exact VID/PID, collection usage, PnP parent and selected container. Revalidate after every reconnect; invalidate cached driver IDs immediately. Reject Pro and ambiguous identities. A matching VID/PID alone does not identify which of two connected V2 units is selected.

Only activate suppression after all native input collections and the output channel are verified. Unknown controls or ambiguous paths block activation. Never discard unrelated strokes. A candidate class filter may observe other devices even when forwarding unchanged; assess its system-wide installation impact explicitly.

Use blocking waits/message pumps, bounded queues and scheduled macro deadlines. Avoid idle polling. Detect queue overflow and delivery failure as faults: stop macros, release owned outputs where possible, disable mappings and expose failure. Lighting work cannot delay input.

## Mapping and output invariants

Assign stable physical IDs only after captures. Model press/release controls separately from wheel impulses. A press stores its resolved action and generation; release unwinds that action even if the layer has changed. Ignore typematic as a new layer-toggle press. Represent extended scan codes explicitly.

Track key/button ownership per physical action and macro invocation. Emit down on first owner and up on last owner, with reverse-order chord cleanup. Reference counts solve overlapping mappings within this process, not physical keyboard ownership across Windows; that remains a backend acceptance gate.

Thumb modes are configurable modifier (default Left Alt), momentary layer stack, or edge-triggered 2/3-layer cycle. On layer transition release current mapped holds, cancel affected macros and quarantine already-held controls until their releases; require a fresh press for a new mapping. Preserve the physical momentary-layer owner until release so the previous layer is restored. This deliberate policy avoids unexpected new keypresses during transitions.

Macros use bounded sequences and delays, cancellation tokens, duration/step limits and explicit held-output ownership. Support keyboard chords, mouse buttons/wheel, media, disabled and layer actions. No arbitrary script execution.

On profile replacement, disconnect, suspend, session lock, backend fault and orderly exit: cancel scheduled actions, release owned output, clear layers, invalidate physical state. Reconnection requires a fresh identity and neutral-state baseline before activation. Exact behavior on forced termination must be established with the backend; do not claim cleanup code covers a crash.

## Configuration and UI

JSON contains schema version, selected device, active profile, profiles/layers/mappings, thumb mode, macro sequences, lighting and startup preferences. Validate complete candidates before applying; preserve last valid configuration after invalid edits/imports. Commit persistence using a same-directory temporary file and atomic replacement. Serialize state changes through the input engine; acknowledge an applied revision to the UI.

Serve only on 127.0.0.1. Validate Host and Origin, require a random per-run token for mutations, disable permissive CORS, cap request sizes and reject path traversal. Browser closure leaves the runtime running. A bounded server-sent event stream can deliver connection and apply status.

The visual keypad must follow verified physical geometry and labels. Include D-pad/diagonals, thumb and wheel selectors, layer editor, focus-only chord recorder, macro timeline, lighting controls and profile import/export. Display capture mode and unsupported controls prominently. Diagnostics mode must never look like active remapping.

## Lighting, startup and distribution

Prefer confirmed hardware effects; only schedule software frames while an effect needs them. Validate colors, brightness, speed and direction; stop animation when disconnected. Save user intent and reapply after reconnect; distinguish disk persistence from device onboard storage. Do not assume a 4x6 protocol matrix has 24 populated LEDs.

Build a Windows GUI-subsystem executable for normal tray startup, with explicit diagnostic output mode. Per-user startup should be opt-in and removable. Tray actions: open configuration, select profile, pause and exit. Runtime remains unelevated wherever feasible. Any privileged driver installer is a separate, explicit action with rollback instructions and approval.

Prefer a small standalone distribution, but do not promise a driver-free executable until suppression is proven. Resolve project and dependency licenses before release; do not import upstream code without attribution and compatibility review.
