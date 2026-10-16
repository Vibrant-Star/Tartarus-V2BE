# Bounded single-key Interception experiment

The user authorized a single-key trial and requested a letter replacement. The explicit `trial` command maps unextended scan 0002 (key 01/native 1) to scan 0030 (physical B). Windows keyboard layout determines the typed character. It does not change stored profiles or install anything.

The trial locates exactly one V2 MI_00 keyboard slot from fresh hardware IDs, refuses ambiguous targets, and selects only that numeric slot in its filter predicate. Other keyboards and mice are not selected. Other strokes delivered from the selected target pass through. The trial checks the target hardware IDs before each received event. This is a bounded experiment, not production reconnect/container handling.

The duration is limited to 60 seconds. The process records only remapped strokes, checks delivery counts, and requests filter removal on completion. It releases an owned B if still held at the deadline. Normal unwinding also attempts release/removal; forced termination, a blocked kernel call, disconnect and sleep are not covered by this guarantee. Keep keys released at startup/end and do not unplug during the experiment. Shared ownership of B with another keyboard remains unverified.

## Filter readback discrepancy

Initial code refused activation because `interception_get_filter` returned 0 after setting mask 3. An independent C# P/Invoke check selected the same single device and observed `before=0 requested=3 after=0 cleared=0`, with 40 predicate calls and 2 matches (set and clear). Administrator elevation did not change that result. Neither check reported a Windows API error.

The subsequent explicit behavioral experiment received and remapped three press/release pairs successfully despite readback 0. Therefore a zero readback is not sufficient evidence that setting the filter failed on this installation. The trial now records this uncertainty and uses actual intercepted events plus an independent observer for validation. This is not a fallback architecture; production remapping remains unavailable. Root cause of the query discrepancy is unresolved, and a zero readback alone cannot prove successful cleanup.

The first observer stopped prematurely due to handling a timer without checking its source window/deadline. This was corrected. The first trial's six successful output writes prove driver delivery, but that incomplete observer cannot establish native suppression or unrelated-keyboard behavior. A repeat with the corrected observer is required.

## Verification

`verify <seconds> <new-trace>` opens a visible test window and records target Raw Input plus ordinary 1/B Windows key messages. For unrelated keyboards, it records only the requested unextended scan 0002 comparison and labels it `other-keyboard`; other typing and unrelated device identities are omitted. The window title reports the last observed 1/B key. Close the window to stop its observer; the separate mapping process still runs until its deadline.

Expected result for three taps on each device: six mapped target B events, no target native 1 events during the trial, and six unchanged non-target 1 events. Ordinary key messages should agree while the window is focused. A final unremapped target key 01 tap after trial completion establishes normal input restoration. Passing this experiment does not establish mouse suppression, macros, layers, shared modifiers, reconnect or crash recovery.

## Verified result — 2026-10-15

The corrected 45-second letter trial passed the observed suppression/replacement test. The user also reported it working. Local evidence: `artifacts/interception/letter-v2-trial.tsv` and `letter-v2-observer.tsv`.

| Observation | Down | Up |
| --- | ---: | ---: |
| Tartarus B, Raw Input | 6 | 6 |
| Tartarus original 1, Raw Input | 0 | 0 |
| Other keyboard original 1, Raw Input | 6 | 6 |
| B, ordinary Windows messages in test window | 6 | 6 |
| 1, ordinary Windows messages in test window | 6 | 6 |

The driver received and wrote exactly 12 mapped strokes, matching the 12 target Raw Input events and 12 B messages. There were no source lookup errors or duplicate target events in this sample. The trial recorded a filter-clear request and `held=false` on completion. Filter readback stayed zero and remains unsuitable as the sole cleanup proof. A post-trial native key tap was not captured, so restoration behavior still needs that explicit check before a persistent runtime is enabled.

This is evidence for one non-modifier key on this Windows installation, not a complete acceptance pass. All five software tests and Clippy checks passed. No persistent remapping configuration, autostart, or additional driver change was introduced.

API references:

- https://github.com/oblitum/Interception/blob/v1.0.1/library/interception.h
- https://github.com/oblitum/Interception/blob/v1.0.1/library/interception.c
- https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input
