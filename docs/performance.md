# Idle resource baseline — 2026-10-16

Measured the deployed release process on this Windows 11 machine with 12 logical processors. The editor/tray remained open in one tartarus-v2be.exe process; the browser was on the observation-only recovery test page, with live view off. The active configuration was native passthrough and the Tartarus was left untouched.

| State | Sample | Process CPU time | CPU, whole-machine capacity | CPU, one logical core | Mean resident working set | Mean private committed memory | Log growth during sample |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Remapping stopped; editor/tray running | 20.22 s | 31.250 ms | 0.0129% | 0.1545% | 11.810 MiB | 2.495 MiB | 0 bytes |
| Remapping active, native mappings | 30.27 s | 15.625 ms | 0.0043% | 0.0516% | 11.883 MiB | 2.508 MiB | 0 bytes |

CPU uses change in cumulative process CPU time divided by measured elapsed time; whole-machine capacity also divides by logical processor count. Short samples and timer quantization mean the lower active result is not evidence that capture reduces CPU use. Working set includes resident shared pages; private memory is committed process memory, not a second amount to add to working set. One-second samples, thread/handle ranges, executable SHA-256 and exact values are in artifacts/performance/idle-results.json. The reproducible observer is Measure-Idle.ps1 in that directory.

Log growth means aggregate application log-file lengths, not OS disk-I/O counters. Startup/shutdown lifecycle entries are outside the sampled steady intervals. Thumb input edges and layer changes still produce diagnostic entries during use; these results do not claim all input-time logging has been disabled. The log folder also retains prior session files.

The observer is a temporary PowerShell process and exits afterward; its overhead, browser CPU/memory and kernel-driver allocations/work outside this process are excluded. These figures do not establish input latency, CPU under sustained input/software lighting/macros, long-term leak behavior or performance with multiple live viewers. The application still uses a blocking 250-ms capture wait and identity revalidation; it is not literally zero-work idle.

The bounded active runtime was stopped and the original configuration remained byte-for-byte unchanged. No resource optimization was applied on the strength of these short measurements alone.
