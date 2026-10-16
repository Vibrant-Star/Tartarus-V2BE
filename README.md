# Tartarus V2BE

<p align="center"><img src="assets/silly-cat.png" alt="A silly cat, just for fun" width="200"><br><em>Just a silly cat. Not an official logo.</em></p>

A lightweight Windows 11 x64 configuration utility and remapping runtime for the **Razer Tartarus V2 (1532:022B)**. Tartarus Pro is excluded. Synapse is not required.

Vibe-coded with AI assistance and refined through hands-on hardware checks. Contributions and independent review are welcome.

**Experimental preview:** core features have local hardware evidence, but full acceptance and broader Windows/security compatibility testing remain incomplete. This is a user-mode runtime using the separately installed Interception kernel drivers for device-specific suppression; Raw Input or global keyboard hooks are not used as a suppression fallback.

## Features

- Independent keypad, D-pad, thumb and wheel mappings; keyboard combinations, media and mouse actions.
- Up to three layers with momentary/cycle thumb behavior and modifier passthrough.
- Bounded macro sequences with configurable holds/pauses and cancellation.
- Local responsive web editor, profiles, combination recording and opt-in live input display.
- Synapse 3 import with original backup preservation and explicit unsupported-item warnings.
- Hardware lighting: static, breathing, spectrum, wave, reactive, brightness and off.
- Optional tray icon, Windows autostart, headless operation, reconnect and sleep recovery.

The browser can be closed independently of the background runtime. Configuration binds to 127.0.0.1 only. Macros, shared-output ownership, recovery and lighting have documented limitations; see the feature documentation before relying on them.

## Run a portable release

Extract the complete portable ZIP into a writable folder. Read START-HERE.md and RECOVERY.md. If Interception is already installed, skip driver installation. Otherwise the explicit driver helper requires administrator access, confirmation and a manual reboot. Double-click Tartarus V2BE.exe for quiet tray startup, or use Open configuration.cmd. Configuration is served at http://127.0.0.1:17832/.

New releases start with native mappings, automatic remapping/autostart off and RGB management off. Back up data/ before updating. Remove the app and the shared Interception driver separately. Never delete driver files or unrelated input filters manually.

## Build and contribute

Install Rust stable on Windows x64. Run cargo build --locked --release; the executable is target/release/tartarus-v2be.exe. See [contributing](CONTRIBUTING.md) for checks and [packaging](docs/packaging.md) for pinned dependency preparation and portable/source archives. Our project source is maintained here and offered separately from the portable download. The Interception library source remains included with the portable dependency notices.

## Documentation

- [Hardware research](docs/research.md) and [architecture](docs/architecture.md)
- [Configuration editor](docs/web-ui.md) and [responsive layout](docs/ui-layout.md)
- [Layers](docs/layers.md), [macros](docs/macros.md), [media](docs/media-outputs.md), [mouse actions](docs/mouse-outputs.md)
- [Synapse import](docs/synapse-import.md), [lighting](docs/lighting.md), [live view](docs/live-view.md)
- [Recovery](docs/recovery.md), [tray](docs/tray.md), [startup](docs/startup.md), [performance baseline](docs/performance.md)
- [Milestones and acceptance gaps](docs/milestones.md)

Some documents retain development-stage measurements and references to local evidence. Those are historical observations, not a promise of identical behavior on every computer. Personal profiles, diagnostic captures, downloaded research sources, dependency binaries and build artifacts are excluded from Git.

## Licensing

Project-authored source is [MIT licensed](LICENSE). The Inter font uses the [SIL Open Font License 1.1](web/fonts/LICENSE.txt). Interception is separately licensed: upstream describes LGPLv3 library/source and non-commercial distribution rights for related driver/installer binaries used through its API. Its public repository does not provide all driver/installer source. Our MIT license does not relicense those components or grant unrestricted commercial distribution rights. Release packages preserve upstream terms, notices and public library source; see [packaging](docs/packaging.md).
