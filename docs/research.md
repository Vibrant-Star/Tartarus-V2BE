# Research baseline — 2026-10-15

## Repository and local environment

The workspace was empty, with no existing application or AGENTS.md found. Rust and Cargo 1.99.0 are available. Host reports Windows build 26200.9711.

Get-PnpDevice failed because CIM access was unavailable. A subsequent read-only `pnputil /enum-devices /connected /deviceids` succeeded and identified a connected 1532:022B composite device, revision 0200. Observed nodes include MI_00 keyboard, MI_01 Col01 keyboard, MI_01 Col02 consumer control, and MI_02. The MI_02 USB interface reports Razer manufacturer, Started status, and oem4.inf. This establishes device presence, not report semantics or remapping compatibility. No drivers, services, or device settings were changed. Existing Razer driver ownership requires investigation before capture and interception testing.

## Hardware evidence and confidence

| Item | Evidence | Remaining work |
| --- | --- | --- |
| Identity | OpenRazer lists V2 as 1532:022B, Pro as 1532:0244 [1] | Match identity and parent/container on each connection |
| USB interfaces | Published V2 descriptor dump has interface 0 keyboard, 1 keyboard, 2 mouse [2]; local enumeration is consistent | Retrieve local HID capabilities and collection relationships |
| Digital input | V2 is the digital target; Pro analog implementation is inapplicable | Capture transitions; do not send Pro unlock commands |
| Every physical control | Exact Windows scan codes, report IDs, and physical associations are unverified | Capture each numbered button, thumb control, wheel and D-pad separately |
| Rollover | No measured rollover guarantee | Measure overlapping keys and modifiers, including more than six keys |
| Diagonals | Representation is unverified | Determine hat state versus pairs of cardinal keys; test transition ordering |
| RGB | OpenRazer V2 class exposes static, breathing, spectrum, wave, reactive, custom rows and off; logical matrix is 4 by 6 [3] | Confirm transport, valid LED positions, timing and responses on Windows |

The published descriptor dump reports endpoint sizes 8/16/8 bytes but explicitly lacks HID report descriptors [2]. Packet size cannot establish NKRO, usages, or physical key count. The requested control inventory must be reconciled against physical labels and captures, avoiding double-counting a numbered thumb control.

## Related implementations

`ultramonaka/open-tartarus-driver` targets **Tartarus Pro**, including analog depth. Its README discusses Interception, but also documents a fallback with unwanted keyboard behavior. Do not copy that fallback or assume V2 input compatibility [4].

OpenRazer is a Linux implementation and a protocol reference, not a Windows interception solution. Its driver has V2-specific command branches [5]. Do not derive a single transaction ID from Pro: the observed V2 brightness branch uses 0x1F while breathing uses 0x3F. Exact report framing, checksum, response checks and interface routing require a command-by-command audit before writes. Record upstream commit hashes before porting protocol code. Review GPL obligations before copying source.

## Windows interception findings

Raw Input provides source device handles [6]. RIDEV_NOLEGACY affects legacy messages for the registering application, not system-wide suppression [7]. A global low-level hook plus asynchronous Raw Input correlation cannot safely establish source ownership and is rejected.

Windows opens keyboard/mouse collections for exclusive system use [8]. HID enumeration or hidapi alone therefore cannot be treated as an exclusive input grab. No verified V2 firmware command that suppresses native output while preserving every control was found in this research.

Interception exposes per-device hardware identification and receive/send operations [9]. It is a candidate for a controlled feasibility experiment, not an approved dependency. Its upstream README says tested through Windows 10 and requires administrator installation [10]. Windows 11 Secure Boot, Memory Integrity, sleep, reconnect and failure behavior remain unverified. Device identification must be proven separately for keyboard and mouse paths; numeric driver slots are not stable identity.

Interception's published licensing distinguishes noncommercial library use and commercial licenses, including access to driver source [10]. This complicates an unrestricted open-source distribution and requires resolution. Do not bundle it by default.

HidHide describes a gaming-device visibility firewall [11]. That description does not prove suppression of this keypad's system keyboard/mouse streams. It remains an unproven alternative, with kernel installation requirements of its own.

If neither validated firmware behavior nor a suitable existing filter meets the requirements, a narrowly scoped KMDF filter will be needed. New kernel drivers require a signing/distribution plan under Microsoft's policy [12], compatibility testing, administrative installation and explicit user approval. No automatic installation or security-setting changes are authorized.

## Blockers before remapping

1. Capture local HID capabilities and correlate all physical controls to source collections/events.
2. Demonstrate selective suppression for every native input path, including both keyboard collections if active.
3. Select a backend with acceptable licensing and verified Windows 11 behavior.
4. Resolve shared modifier ownership: SendInput state is not per physical keyboard. Application reference counting alone cannot guarantee that releasing virtual Alt leaves another keyboard's held Alt intact. Test overlapping down/up orderings before accepting modifier passthrough.
5. Establish recovery after process crash. Normal shutdown cleanup cannot run after termination or power loss; define backend timeout/fail-open behavior and demonstrate how virtual held states recover.

SendInput is subject to UIPI and existing key state [13]. Elevated applications, secure desktop and games with input restrictions cannot be promised universal compatibility. Backend failure must expose an explicit unavailable state; native input may remain available, but this must never be presented as successful remapping.

## Sources

All accessed 2026-10-15; upstream main/master references are mutable.

1. https://github.com/openrazer/openrazer
2. https://github.com/openrazer/openrazer/issues/983
3. https://github.com/openrazer/openrazer/blob/master/daemon/openrazer_daemon/hardware/keyboards.py — RazerTartarusV2
4. https://github.com/ultramonaka/open-tartarus-driver/blob/main/README.md
5. https://github.com/openrazer/openrazer/blob/master/driver/razerkbd_driver.c
6. https://learn.microsoft.com/en-us/windows/win32/inputdev/about-raw-input
7. https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-rawinputdevice
8. https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/keyboard-and-mouse-hid-client-drivers
9. https://github.com/oblitum/Interception/blob/master/library/interception.h
10. https://github.com/oblitum/Interception
11. https://github.com/nefarius/HidHide
12. https://learn.microsoft.com/en-us/windows-hardware/drivers/install/kernel-mode-code-signing-policy--windows-vista-and-later-
13. https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput
