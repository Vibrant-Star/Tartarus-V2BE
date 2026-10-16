# Mouse output assignments

The editor now offers **Mouse button** (left, right, middle, back, forward) and **Mouse wheel** (up, down, left, right) actions for every remappable physical control. Existing schema-version 1 files remain valid.

~~~json
{
  "schema_version": 1,
  "mappings": {
    "01": {"type": "mouse_button", "button": "left"},
    "02": {"type": "mouse_wheel", "direction": "down"}
  }
}
~~~

Buttons hold until the physical control releases. Physical keyboard repeats do not generate repeated mouse-button downs. Two mapped controls sharing a button send one down and the last owner's up; a wheel notch mapped to an already-held button does not click/release that other owner's hold. Layer/profile changes and normal stop release owned button outputs and quarantine old physical holds.

Wheel actions are one 120-unit step per press or input-wheel notch. Keyboard repeats can produce more steps while a mapped key is held. Horizontal left is negative and right positive. Wheel actions have no release or owned hold state. This increment does not add timed continuous scrolling, pointer movement, mixed keyboard/mouse chords or macros.

Internally, mouse outputs use a reserved tagged namespace, separate from keyboard scan codes and physical wheel/middle input IDs. Only the backend converts these to Interception mouse packets on the identified Tartarus mouse slot; internal values are refused by the keyboard emitter. Output packets contain no motion, carry the recursion marker, and use [Interception's published button/wheel fields](https://github.com/oblitum/Interception/blob/master/library/interception.h). No driver installation, global hook, added dependency or background process is involved.

If any layer emits middle-click, native Tartarus middle-click is routed through the ownership engine across all profile layers unless explicitly remapped or disabled. This prevents a mapped middle release from releasing another native Tartarus middle hold. Other physical mice are not intercepted. Windows' shared button state across a separate real mouse and injected output remains a hardware acceptance gate; this increment does not claim that reference counting solves it. Existing disconnect/sleep recovery and forced-termination limitations remain.

## Validation on 2026-10-16

40 software tests pass; one interactive Explorer lifecycle test remains opt-in. New coverage checks all button and wheel packet encodings, absence of pointer motion, independent wheel impulses, button repeat suppression and shared ownership, native Tartarus middle ownership, strict JSON rejection, and cleanup/quarantine on layer changes and configuration reload. Formatting, Clippy with warnings denied, JavaScript syntax and optimized build pass. The filesystem can emit a hard-link cache warning; this is unrelated to source lint warnings.

Browser verification assigned mouse right and wheel down to key 01 as drafts, verified the rendered controls, then restored the saved native draft without applying it.

A bounded test used 01 left, 02 right, 03 middle, 04 wheel up, 05 wheel down. The first attempt refused startup because a key was held and restored the original configuration. After the user explicitly confirmed all keys/buttons released, the retry started. The user confirmed all five actions worked and a held left button cleared on release. The local test page observed matching down/up counts (left 5/5, right 2/2, middle 2/2), two negative and two positive vertical scroll events, and no held buttons. This page also counts real-mouse activity, so it is supporting observation rather than independent source attribution. Back/forward, horizontal wheel and real-mouse shared-button overlap remain hardware follow-up checks.

The local /mouse-test page observes browser events only in memory, prevents default clicks/context menus/scrolling in its test box, and never starts the runtime. No observations are saved or transmitted by that page. Side buttons can still cause browser navigation and are excluded from the announced test. Explicit test artifacts are under artifacts/mouse-test and artifacts/live-view.
The completed runtime reported 22 input packets, owned=0 and filter-clear requested. Stop was confirmed with no runtime error; the active configuration exactly matches the saved original after restoration.
