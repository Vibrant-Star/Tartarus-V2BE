# Macro sequences

Select any physical control in the editor, choose **Macro sequence**, and edit its steps in the wide horizontal editor. Use **Add step**, **Move left/right** and **Remove** to arrange keyboard combinations, mouse-button taps, mouse-wheel notches and pauses. Key names match ordinary assignments; for example `LeftCtrl + C`. **Hold (ms)** controls a tap's duration; **Pause (ms)** waits between actions. Each tap automatically releases in reverse order. Update the assignment, then Apply changes. Profiles, duplication, JSON import/export and layers retain macro assignments.

Playback starts once on a fresh physical press. Releasing the trigger allows the sequence to finish; holding it does not repeat. A second press or wheel notch while that same control's sequence is running is ignored rather than queued or restarted. Different controls can play concurrently. A wheel control starts one sequence per eligible notch. There are no infinite loops, toggle playback, arbitrary scripts or unbalanced key-down/key-up steps in this version.

Changing layer or profile cancels all sequences and releases their held outputs. Stop, disconnect, suspend and normal shutdown use the existing delivered-output cleanup path. Held physical triggers must be released before being used again after layer/profile transitions. Keyboard modifiers, mouse buttons and ordinary mappings share the same Tartarus output ownership counts, including native Tartarus middle-click where applicable. Overlapping taps of an already-owned output may be coalesced; they do not release another mapping's hold. Existing limitations for shared Windows state with unrelated physical devices and forced process termination still apply; macros do not add interception of other devices.

The runtime advances playback on its existing input thread. It waits until the next step deadline or the existing 250-ms control timeout, whichever is sooner. It creates no per-macro threads or busy polling. Each sequence is limited to 128 steps and 60 seconds of planned time; individual holds/pauses are 1–10000 ms. Keyboard taps accept 1–8 distinct keys. Timing is best effort: delayed scheduling preserves the next full hold/pause rather than racing through overdue steps. Actual playback can therefore take longer than planned. No per-step logging is added.

## JSON example

```json
{
  "schema_version": 1,
  "mappings": {
    "01": {
      "type": "macro",
      "steps": [
        {"type": "tap", "keys": ["A"], "hold_ms": 40},
        {"type": "delay", "ms": 300},
        {"type": "tap", "keys": ["B"], "hold_ms": 40}
      ]
    }
  }
}
```

Other steps are `{"type":"mouse_button","button":"left","hold_ms":40}` and `{"type":"mouse_wheel","direction":"up"}`. Button names are left/right/middle/back/forward; wheel directions are up/down/left/right. Omitted tap hold time defaults to 40 ms. Schema-version 1 remains compatible with existing profiles. Preserved Synapse macros remain passive metadata; this feature does not interpret their format or claim Synapse macro compatibility.

## Validation

Automated coverage checks sequence order, delays, late scheduler wakes, overlapping playback, shared ordinary/macro ownership, restart suppression, cancellation, layer/profile quarantine, thumb-mode changes, wheel triggers and validation limits. The user confirmed an actual Tartarus key 01 produced A, then B after a pause, and holding the trigger produced only one sequence. The bounded test restored the exact saved user configuration afterward. Mouse/media macro combinations and cancellation during actual disconnect/sleep still need dedicated hardware validation. The local `/macro-test` page is an optional typing box, not a global input recorder.
