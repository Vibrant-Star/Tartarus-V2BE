# Synapse 3 import (first supported format)

Use **Import Synapse 3** in the editor and select a `.synapse3` backup. The server reads it entirely inside the existing Rust process; it does not launch Synapse, PowerShell, another helper, or a driver installer. Import is on demand, with no extra idle worker. The updated keypad schematic follows the supplied device reference: 19 keys on the main block, wheel beside key 19, small HyperShift control above the D-pad, and the separate key 20 at the far lower right. These are existing control identities, not new input assignments.

The preview lists every exported profile/keymap and marks the keymap selected in its source profile. Base and HyperShift assignments appear side by side. **Load selected keymap as draft** does not activate it. Save to library keeps it for later; **Apply changes** replaces the active configuration. Start remapping remains a separate action when the runtime is stopped.

## What converts

- Only backups identifying Tartarus V2 VID 5426 / PID 555 (`1532:022B`) are accepted. Tartarus Pro is rejected.
- The captured default HID usages identify controls 01–20 and the thumb; mappings do not infer physical identity from remapped output values. A few physical labels remain provisional from the earlier capture.
- Ordinary keyboard assignments, recognized Left/Right Ctrl/Alt/Shift/Win combinations, and confirmed E0 `Extended_00` scan codes convert to existing runtime actions. Digits, Home/End/PageUp/PageDown/Insert/Delete and Minus now have readable names as well as raw scan-code support.
- Copy/Cut/Paste convert to Ctrl+C/X/V. Other Windows shortcut actions are not guessed.
- Wheel actions that retain their native role remain passthrough; keyboard assignments on wheel controls convert normally. Other mouse output actions are not yet supported.
- A thumb HyperShift trigger on both source layers becomes a momentary HyperShift layer. Conflicting trigger behavior blocks loading; layer transitions retain our existing release/quarantine policy rather than claiming exact equivalence with every Synapse behavior.
- Confirmed `DIRECTION` D-pad assignments convert. The inactive `BUTTON_*` joystick entries are counted and retained, without enabling joystick output. Other D-pad modes block loading. Compass orientation remains uncalibrated; names refer to captured arrow signatures.

Unsupported active actions/options are explicitly listed and their recognized controls are disabled in the draft. Unknown physical inputs, duplicate assignments, unsupported D-pad modes and incompatible thumb behavior block loading that keymap. Unknown mapping-list structure, malformed archives/XML, other device models and configurations exceeding our limits reject the preview. This is intentionally narrower than universal Synapse compatibility.

## Preservation

Each converted JSON includes optional `source_import` metadata containing format, original filename, profile/keymap identifiers, and **the exact original compressed archive encoded as hexadecimal**. That retains inactive keymaps/settings, lighting, macros, unknown feature files and device identifiers without attempting to execute them. Normal configuration validation checks metadata shape/size; it does not interpret or run the embedded archive. UI edits, library saves and JSON exports retain this metadata.

The original file is never modified. JSON export carries the original backup and its identifiers, which is stated in the UI. Retaining source bytes is not a claim that Synapse export/round-trip serialization works. Lighting, polling, idle power, gaming mode, linked applications and macros are not activated by import in this increment.

The current archive limit is **16 KiB compressed**, keeping the hex copy and converted mappings within the existing 64 KiB JSON configuration limit. Up to 128 ZIP entries, 512 KiB per expanded entry, 1 MiB total expanded content, 30,000 XML nodes per document and 16 converted keymaps are accepted. XML is UTF-8; DTD/external entity processing is disabled. ZIP entries are read in memory, not extracted to paths. Unsafe/duplicate path names, encryption and unsupported compression are rejected. Larger backups need a future separate-source-storage design; the importer reports this limit rather than discarding preserved data.

Parser dependencies are [zip](https://docs.rs/zip/9.0.3/zip/) with only the flate2 Deflate backend plus Stored support, and [roxmltree](https://docs.rs/roxmltree/0.21.1/roxmltree/struct.ParsingOptions.html). Defaults enabling other compression/encryption families are disabled. Dependency versions/checksums are locked. This changes executable size but adds no process, continuous logging or background parsing.

## Validation — 2026-10-15

The supplied `example.synapse3` is 5,314 bytes and contains profile Example with Default and Example keymaps. Both previews convert 56 assignments, retain 16 inactive D-pad entries, and report no unsupported active assignment warnings. Example is marked selected in the source. Its root thumb becomes momentary layer 1. Examples: base key 02 → 3, wheel click → Insert, and D-pad → Home/Down/Delete/End; HyperShift key 01 → Alt+Tab and 17–19 → Copy/Cut/Paste.

Thirty software tests pass, including synthetic archive conversion, extended keys/chords, exact source byte preservation, inactive D-pad exclusion, thumb trigger conversion, model/path/DTD/size rejection, unsupported-action disabling, and unknown/duplicate input blocking. Existing engine and local-server tests pass, as does Clippy with warnings denied. Browser checks previewed the real backup, loaded the selected keymap, saved it as library profile **Example**, and displayed both layers. Original archive bytes recovered from the saved metadata matched the source SHA-256, and the saved profile passed the production configuration validator.

Active configuration remained native-only and remapping stopped. The imported profile has not yet been exercised on hardware; broad cross-keyboard modifier, reconnection and sleep reliability gates remain open. Generated preview/profile data includes private device/profile information and stays in ignored `artifacts/` and `data/` directories, not committed fixtures.

A read-only CLI is also available: `tartarus-v2be preview-synapse <backup-path>` emits the preview as JSON and never opens an interception device.

Final browser verification reopened the saved Example library profile after restart, confirmed key 20 selects its own Space assignment in the new lower-right location, and reported no browser warnings/errors. Evidence: `artifacts/synapse-import/preview.json` and `layout-example.jpg`. The final optimized executable is 982,528 bytes. The imported profile is left visible as an unapplied draft; disk active configuration is unchanged and the runtime is stopped.
