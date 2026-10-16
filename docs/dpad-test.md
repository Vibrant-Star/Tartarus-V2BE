# Orientation-independent D-pad verification

The user requested a functional test without assigning directions to the D-pad's physical surface. The test maps the four native extended arrow signatures to distinct A/B/C/D outputs. Logical `dpad_up/down/left/right` names in the JSON refer to captured Windows arrow codes, not confirmed physical positions.

`scripts/Test-Dpad.ps1` starts a visible passive observer and a bounded keyboard runtime. Its temporary configuration changes only the four D-pad signatures; numbered keys remain native. The user can tap available positions, roll around, hold between positions, combine a D-pad hold with a numbered key, and compare an arrow on a separate keyboard. The harness requests stop after about 65 seconds, with an 80-second runtime safety deadline, and keeps observing for a restoration check.

The keyboard backend now requests all keyboard stroke classes on the selected Tartarus MI_00 slot, including E0/E1 extended strokes. Unhandled special strokes are forwarded. The previous mask was limited to ordinary down/up flags, so earlier number-key success could not establish D-pad coverage. No filter is enabled for unrelated keyboards or mice.

The visible observer records A–D and native arrow Windows messages. For unrelated devices, only the explicitly requested comparison 1/arrow events are retained under an anonymous source label; unrelated typing and hardware identities are omitted.

Checks:

- All four distinct replacements occur with valid press/release pairing.
- No original Tartarus arrow events escape during active remapping.
- Rolling and returning to neutral leave no replacement keys held.
- Overlapping replacement letters establish whether two arrow components coexist; this does not by itself implement separate diagonal mappings or identify a physical compass orientation.
- A numbered key can coexist with D-pad output.
- A separate keyboard arrow remains unchanged.
- After stopping, the Tartarus emits native arrows again.

Repeats while held are normal Windows typematic events, not automatically evidence of duplicate injection. Compare Raw Input and ordinary message streams, and distinguish active remapping from the post-stop interval. Software tests now include an extended-key diagonal transition with independent output ownership; physical behavior requires the saved trace.

## Result — 2026-10-15

Passed for the sampled behavior. Evidence: `artifacts/interception/dpad-20261015-203736/`, including the temporary JSON, runtime log, observer trace and generated summary. The user noted multiple arrow taps on the regular keyboard; source separation kept those distinct from target events.

| Replacement | Presses | Releases |
| --- | ---: | ---: |
| A | 8 | 8 |
| B | 7 | 7 |
| C | 10 | 10 |
| D | 4 | 4 |

Raw Input and ordinary Windows message counts agree for all letters. There were no original target arrow events during remapping, no unmatched replacement releases, no held replacement keys at completion and zero source lookup errors. Eight unchanged arrow events came from the regular keyboard. Twelve numbered-key 01 events occurred while at least one D-pad replacement was held, establishing coexistence in this sample.

All four expected two-component combinations occurred: A+C, A+D, B+C and B+D. Total observed overlap per pair was approximately 0.38–0.67 seconds. This supports digital diagonal combinations, without assigning a physical compass orientation or proving separate diagonal-action mappings.

After graceful stop, 43 native target arrow events were observed, including repeats/transitions; no native arrow keys remained held at the observer's end. The runtime recorded 76 received inputs, `owned=0` and a filter-clear request on stop. The summary uses the runtime log's final-write timestamp as an approximate stop boundary; tested replacement and restoration events are well separated from it.

All 13 software tests, Clippy and the optimized build pass. This functional test does not establish sleep/reconnect behavior or long-run reliability. No persistent D-pad remapping was enabled.
