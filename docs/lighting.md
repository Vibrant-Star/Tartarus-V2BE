# Hardware lighting

The configuration page now offers device-wide Off, Static, Breathing, Spectrum cycling, Wave and Reactive keypress effects, brightness, color where supported, reactive duration and two wave directions. Select **Control Tartarus lighting** and **Apply lighting**. Lighting is independent of input profiles and does not require remapping to be running. Closing the browser leaves hardware effects running.

Settings persist in `data/lighting.json`. Disabling management leaves the current effect untouched; choosing Off darkens the backlight. Managed settings are reapplied on application startup and after device/power notifications. An error remains visible if the device cannot be reached; input handling does not silently switch architectures. Reconnection and sleep restoration of lighting still need hardware validation.

## Confirmed on this Tartarus V2

The user visually confirmed static blue, brightness changes, Off, single-color breathing, moving rainbow wave and reactive blue keypress fading. Spectrum cycling was the original effect and was restored at its queried original brightness of 20%. Effect-setting requests were acknowledged by the device and brightness was read back. There is no reliable current-effect readback implemented. Wave directions use neutral labels because their physical orientation has not been calibrated. Per-key custom frames, multiple breathing colors and profile indicator LEDs are outside this version's controls.

## Device access and protocol

This implementation targets only Razer Tartarus V2 VID/PID `1532:022B`, excluding Tartarus Pro. Native Windows HID enumeration found its MI_02 mouse collection with usage page 1, usage 2 and 91-byte feature reports. Each operation verifies VID/PID and requires exactly one matching control collection; ambiguous devices produce an error. Lighting uses feature reports, without reading input reports or changing the installed Interception driver.

Protocol facts were cross-checked against [OpenRazer](https://github.com/openrazer/openrazer/tree/477f4d6d3ab7e9deb6310d5bd3742560c5cbe80d) and [OpenRGB](https://github.com/CalcProgrammer1/OpenRGB/tree/6a10f77c15e4be47c792b89f1a3cc4e992389684). Reference snapshots and local diagnostics are under `research/rgb/`. Project-authored packet encoding is independent of the reference implementations.

Windows reports contain report ID zero followed by the 90-byte Razer packet. The V2 uses transaction `0x1F`, extended lighting class `0x0F`, effect command `0x02`, brightness command `0x04` and brightness query `0x84`. Replies are checked for matching transaction/command, length, checksum and success status. All writes use volatile storage zero; the application persists desired settings rather than repeatedly writing device flash.

The device animates these effects itself. A dedicated worker sleeps between configuration/device/power events, with bounded backoff after failures; there is no animation polling loop or per-key logging. Browser status polling reads cached state, without HID queries. Hardware I/O remains synchronous inside the worker, so a misbehaving Windows HID stack can delay that worker's shutdown. Do not run another RGB controller for the same device concurrently; the application's mutex coordinates only Tartarus V2BE processes.

## Validation and limits

Protocol encoding, response rejection, bounds, persistence and stale-edit handling have automated coverage. The build passed 50 tests with one unrelated Explorer lifecycle test ignored, formatting, strict Clippy and JavaScript syntax checks. A graceful application restart loaded saved spectrum/20% settings and successfully reapplied them, including brightness readback. Hardware evidence is saved in `artifacts/rgb-test/`. These results establish the effects tested above, not complete project acceptance or exhaustive RGB recovery testing. Previous idle performance measurements predate lighting support.
