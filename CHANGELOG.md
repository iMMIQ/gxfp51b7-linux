# Changelog

## 0.3.0 — 2026-10-08

- Consolidated authentication on ChicagoHS, libfprint TOD and standard fprintd/PAM.
- Removed affine NCC, RootSIFT, the custom PAM crate and duplicated research tooling.
- Added independent background calibration and standard PAM commissioning.
- Pinned Rust 1.99.0 and aligned Rust, C and Python quality checks with CI.
- Hardened worker events and added malformed-stream/cancellation tests.

## 0.2.0 — 2026-10-08

- Introduced the Rust capture worker and guided 12-position ChicagoHS enrollment.
- Integrated standard fprintd and validated actual SDDM authentication/account checks.

## 0.1.0 — 2026-10-08

Initial public-source preparation for GXFP51B7 Linux support.

- ACPI mailbox transport, read-only BIOS helper and guest-isolated signed-enclave capture.
- 80×64 image decoder and bounded affine NCC matcher.
- Optional SDDM PAM branch with timeout and password fallback.
- Parameterized private-bundle installation, local enrollment, live validation and rollback tools.
- English/Chinese README, reproduction/design/security documentation and offline CI.

Hardware results apply to the original deployment. Clean-installation qualification of the generalized tooling is the next validation step.
