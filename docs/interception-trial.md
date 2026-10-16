# Interception feasibility trial

Selected for evaluation by the user on 2026-10-15. The user subsequently approved kernel installation with rollback preparation. Installation completed at 19:54 Moscow time. After the user restarted Windows, both drivers reported RUNNING and the probe identified two V2 keyboard paths and one V2 mouse path. Physical control coverage and suppression remain unverified; see [diagnostics](diagnostics.md).

## Prepared package

- Source: https://github.com/oblitum/Interception/releases/tag/v1.0.1
- Download: https://github.com/oblitum/Interception/releases/download/v1.0.1/Interception.zip
- Local archive: `vendor/interception/Interception.zip` (excluded from source control)
- SHA256: `AD038963D6413055765128B0B931F6E765147C9916DBA79E65D872B261F9AF10`
- Local installer: `vendor/interception/v1.0.1/Interception/command line installer/install-interception.exe`
- Windows Authenticode inspection reports NotSigned for both installer and x64 user-mode DLL. This does not establish the signing status of embedded kernel drivers. The hash records the downloaded bytes; it is not independent publisher verification.

## Implemented probe

Dependency-free Rust x64 executable with explicit absolute DLL loading and restricted dependency search. It opens an Interception context, enumerates hardware IDs for slots 1–20, prints only candidate V2 IDs, and closes the context. It never calls filter-setting, receive or send APIs. Opening the context registers driver events but enables no capture filters. A loaded DLL executes native code, so use the upstream package rather than an arbitrary DLL.

Build and run in PowerShell from the project directory:

```powershell
cargo test
cargo build --release
& .\target\release\tartarus-v2be.exe probe 'D:\Apps\Tartarus V2BE\vendor\interception\v1.0.1\Interception\library\x64\interception.dll'
```

Current local result: DLL loaded, but context creation failed. The API cannot distinguish absent drivers from blocked/inaccessible/in-use drivers. No candidate slots can be verified yet. No interception or injection was enabled.

Hardware IDs alone do not distinguish identical units or prove collection coverage. A successful probe is only an inventory milestone, not authorization to suppress input. Container correlation, keyboard and mouse coverage, and controlled input forwarding tests remain required.

## Installation — completed and running after reboot

Necessity: Interception's kernel filters provide the suppression path that Raw Input lacks. Installation affects the system keyboard/mouse stacks, including unrelated devices at the filter attachment level, even though the eventual runtime must select only the Tartarus. Administrator access and a reboot are required. Windows 11 compatibility and security-feature compatibility are not established by upstream's Windows 10 test claim. Keep Secure Boot and Memory Integrity settings unchanged; stop if installation is blocked.

After explicit approval, run this exact local installer with `/install` elevated, save its output, and ask the user to reboot when convenient. Do not reboot automatically. After reboot, rerun the read-only probe before enabling any capture.

Rollback uses the same installer with `/uninstall`, elevated, followed by a user-controlled reboot. Do not remove arbitrary keyboard/mouse driver files manually. A failed installation or input-stack conflict can require Windows recovery; the trial should occur when a restart is acceptable.

The approved upstream installer returned exit code 0 and reported successful installation with reboot required. Verification found added `keyboard`/`mouse` class UpperFilters ahead of the preserved `kbdclass`/`mouclass`, corresponding services, and driver files. Both installed .sys files report Valid Authenticode signatures from Francisco Lopes da Silva. This does not prove Windows will load them under the machine's security policy after reboot.

The probe still cannot create a context before reboot, as expected for the pending installation; post-reboot compatibility is unverified. No reboot, security-setting change, startup registration or input capture was performed. The baseline, installer output and before/after state are saved under `artifacts/interception/`. See [recovery instructions](interception-recovery.md) and the project-root `Uninstall-Interception.cmd` launcher. Distribution licensing remains unresolved; the upstream package is a local evaluation dependency and is not bundled with project source.
