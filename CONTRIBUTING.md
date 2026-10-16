# Contributing

Target Windows 11 x64 and Tartarus V2 VID/PID 1532:022B only. Do not assume analog Tartarus Pro capabilities or replace device-specific interception with global suppression hooks. Keep kernel driver installation explicit and separate from development/tests.

Use Rust stable with rustfmt and Clippy. Run `cargo fmt -- --check`, `cargo test --locked`, `cargo clippy --locked -- -D warnings`, `node --check web/app.js` and `node --check web/layout.js`. The Explorer lifecycle test is ignored by default because it needs an interactive notification area. Automated checks do not establish hardware compatibility or authorize installing/removing a driver.

See docs/packaging.md to prepare pinned upstream inputs and build portable/source archives. Keep runtime profiles, Synapse backups, HID paths and raw captures out of Git. Report reproduction steps and a redacted error summary rather than uploading personal configuration or full system traces by default.

Project-authored changes are MIT licensed. Preserve dependency notices and do not copy GPL implementation code into the MIT source without reviewing the resulting licensing obligations. Protocol references and driver binaries are separately licensed dependencies.
