# GXFP51B7 Linux

[简体中文](README.zh-CN.md) · [Installation](docs/INSTALL.md) · [Design](docs/ARCHITECTURE.md) · [Validation](docs/VALIDATION.md)

Linux fingerprint login for **Huawei MACHC-WAX9 / ACPI GXFP51B7**.

A Rust capture worker connects the EC sensor to the ChicagoHS matcher and
libfprint TOD. Standard fprintd manages enrollment and standard `pam_fprintd`
authenticates SDDM. Enrollment collects 12 accepted positions of the same finger.

## Supported configuration

| Component | Configuration |
| --- | --- |
| Laptop / BIOS | Huawei MACHC-WAX9 / 1.24 |
| CPU | Intel Core i7-10510U, SGX enabled |
| Sensor / firmware | ChicagoHS `0x2504` / `GF_9ELIBE_EC_19024` |
| Host | EndeavourOS x86_64, kernel `7.2.9-arch1-1` |
| Login manager | SDDM 0.21, Arch `system-login` PAM layout |
| Isolated runtime | QEMU/KVM 11.1.2, Ubuntu 20.04, kernel `5.4.0-216-generic` |

Installation validates the machine identity and reserved ACPI resources.

## How it works

The ACPI/EC mailbox carries encrypted sensor traffic. Original signed Goodix
and Intel enclaves run inside a same-machine KVM guest and validate captures.
Rust decodes the 80×64 image, checks contact quality and calls the pinned
ChicagoHS algorithm. libfprint and fprintd handle print serialization, storage,
progress, cancellation and authentication access control.

The communication key stays inside the vendor enclave. Captures are processed
in memory; calibration and enrollment files use root-private storage.

## Build and quality checks

The project pins **Rust 1.99.0** in `rust-toolchain.toml`. Install a C compiler,
GLib/GIO and libfprint TOD development headers, pkg-config and PAM development
headers. Python 3.11+ supplies the offline development tools.

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r requirements-tools.txt
cargo install cargo-machete --version 0.9.2 --locked
make check
```

`make check` builds the release executable, adapter and probes, runs Rust and
native tests, checks independent decoding/quality references, and runs Rust/C/Python
formatting, strict Clippy, Ruff, dependency usage and documentation checks.
`make kernel` and the guest build cover their platform-specific components.

## Installation and use

The [installation guide](docs/INSTALL.md) covers the private vendor bundle,
SGX guest, host helpers, empty-sensor calibration, fprintd enrollment, live
qualification and SDDM enablement. The [fprintd guide](docs/FPRINTD.md) describes
the adapter and standard client interfaces.

```sh
fprintd-enroll -f right-index-finger YOUR_ACCOUNT
fprintd-verify -f right-index-finger YOUR_ACCOUNT
```

After enabling SDDM, select the enrolled account, submit an empty password and
touch the enrolled finger. Password authentication remains available following
a mismatch or timeout. The tested eos-breeze theme supports this flow.

## Repository

```text
crates/core/     decoder, quality checks and background compatibility reader
crates/backends/ Rust ChicagoHS wrapper and pinned upstream algorithm
crates/driver/   transport, calibration, commissioning, installer and VM commands
fprint/         libfprint TOD adapter, API probe and worker transport tests
pam/            standard PAM authentication/account probe
kernel/         BIOS sealed-container reader and DKMS configuration
guest/          signed-enclave ABI loader and guest bootstrap
tools/          vendor asset verification, native tests and release checks
tests/          independent synthetic decoder/protocol/quality references
data/           service configuration
docs/           installation, architecture, security and validation records
```

Captures, prints, keys, BIOS containers, vendor components and VM disks reside
in private local storage. Public issues contain sanitized metadata.

## License and credits

Original user-space code uses **LGPL v3 or later**. The kernel helper offers
**GPL-2.0-only OR LGPL-3.0-or-later**. Vendored ChicagoHS source retains its
LGPL-2.1-or-later grant. See [license scope](LICENSE.md),
[third-party notices](THIRD_PARTY_NOTICES.md) and the
[pinned algorithm provenance](crates/backends/native/UPSTREAM.md).
