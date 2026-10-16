# Media key outputs

Choose **Keyboard / media key** in the assignment editor, then enter or select a suggestion: VolumeUp, VolumeDown, VolumeMute, MediaPlayPause, MediaStop, MediaNextTrack or MediaPreviousTrack. Update assignment edits the draft; Apply changes activates it. JSON uses the existing key action, for example:

~~~json
{"schema_version":1,"mappings":{"01":{"type":"key","key":"MediaPlayPause"}}}
~~~

These use E0 set-1 keyboard strokes through the existing selected-device Interception output path. Codes follow [Microsoft's Keyboard Input Overview](https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input): previous 10, next 19, mute 20, play/pause 22, stop 24, volume down 2E, volume up 30 (hexadecimal, all extended). No separate media process, global hook, driver installation or additional dependency was introduced. Playback behavior depends on Windows and the receiving media application.

Keyboard holds retain existing repeat behavior; a wheel notch sends a down/up pulse. Shared output ownership, profile/layer cleanup and quarantine use the existing engine. The combination recorder recognizes standard browser media key codes when the browser exposes them; Windows or the browser may consume those keys first, so manual assignment remains available.

Validation on 2026-10-16: 35 software tests pass, one Explorer lifecycle test remains opt-in. All seven media outputs have distinct expected extended strokes, share ownership correctly between two controls and release on reload without a later duplicate release. Clippy with warnings denied, JavaScript syntax and optimized build pass. The deployed authenticated validation endpoint accepts all seven names. Browser verification assigned VolumeUp to key 01 as a draft and then reloaded the saved native configuration; no mapping was applied or runtime started. Actual Windows volume/playback delivery from the Tartarus remains a hardware acceptance check. Remapping is stopped.

## Bounded hardware check

The user confirmed that key 01 lowered Windows volume and key 02 raised it, while the regular keyboard behaved normally. The test used a 60-second deadline with only those two explicit mappings. This confirms the volume up/down route in this environment; mute, playback/track controls and broader application compatibility remain untested on hardware. Test setup and restoration evidence is retained under artifacts/media-test.
