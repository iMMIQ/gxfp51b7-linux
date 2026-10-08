# Contributing

Use the pinned Rust 1.99.0 toolchain. Install `requirements-tools.txt` and
`cargo-machete`, then run `make check` before submitting a change. `make fmt` applies Rust/C/Python formatting;
`make lint` checks Clippy, Ruff and unused direct dependencies. Install `cargo-audit` 0.22.2 and run `make audit` for the current RustSec dependency audit. Keep hardware-independent checks based on source and synthetic fixtures, runnable as a regular user. Describe the problem, changed behavior and validation performed. Distinguish compile/offline results from real hardware evidence.

Protocol changes need bounds and malformed-input coverage. Authentication changes must preserve the password fallback and account restrictions. Matcher changes need independent same-finger/different-finger evaluation; keep development captures separate from held-out evaluation. Threshold changes require independent acceptance and rejection evidence.

Hardware support reports should identify model, BIOS, ACPI ID and software versions, with sanitized metadata. Keep fingerprint images, templates, private keys, sealed BIOS data, vendor components and guest disks in private local storage. Public reports should contain sanitized metadata and reproducible steps.

Contributions use the per-file license shown in the source. Preserve third-party notices and identify any imported code and its license. Contributions should follow the repository’s source-based structure: independently written host/guest glue, matching code, tests and documentation.
