# GXFP51B7 Linux

[简体中文](README.zh-CN.md) · [Installation](docs/INSTALL.md) · [Design](docs/ARCHITECTURE.md) · [Validation](docs/VALIDATION.md)

Rust implementation of Linux fingerprint acquisition and SDDM authentication for the **Huawei MACHC-WAX9** with ACPI device **GXFP51B7**.

The project provides an encrypted capture backend, fingerprint matcher and PAM module for SDDM login. Host operation uses a compiled Rust executable and Rust PAM library.

## Supported configuration

| Component | Configuration |
| --- | --- |
| Laptop / BIOS | Huawei MACHC-WAX9 / 1.24 |
| CPU | Intel Core i7-10510U, legacy SGX launch flow |
| Sensor / firmware | ChicagoHS `0x2504` / `GF_9ELIBE_EC_19024` |
| Host | EndeavourOS x86_64, kernel `7.2.9-arch1-1` |
| Login manager | SDDM 0.21, Arch `system-login` PAM layout |
| Isolated runtime | QEMU/KVM 11.1.2, Ubuntu 20.04, kernel `5.4.0-216-generic` |

Installation checks the machine identity and its ACPI resources against this configuration.

## How it works

An ACPI/EC mailbox carries the sensor's encrypted traffic. A small host module reads the BIOS-sealed communication container. Original signed Goodix and Intel enclaves run inside a same-machine KVM guest, unseal the communication key and validate the captured data. The host decodes an 80×64 image and compares it with a private local template. A bounded PAM helper supplies an optional fingerprint branch before the existing password path.

The communication key stays in the vendor enclave. Enrollment templates reside in a root-private directory; normal login capture returns image data in memory.

## Build and test

Install Rust 1.88 or newer, a C compiler, PAM development headers and Python 3.11 or newer for offline tooling, then:

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r requirements-tools.txt
make check PYTHON=python
```

These offline checks cover source compilation, packet parsing, image decoding, matching mathematics and PAM configuration generation. The guest loader and host kernel helper are separate build targets; see the [installation guide](docs/INSTALL.md).

## Installation and use

Installation combines this repository’s source with user-supplied vendor components, a prepared SGX guest and an SSH identity unique to that guest. The [guide](docs/INSTALL.md) documents preparation, host installation, enrollment, live validation, SDDM enablement and rollback. The installation workflow targets the MACHC-WAX9 and uses locally prepared assets.

After enrollment and successful live validation, select the enrolled account in SDDM, submit an empty password and touch the enrolled index finger. A mismatch, unavailable backend or timeout falls through to password authentication. Some SDDM themes may require a separate UI adjustment; the tested eos-breeze theme accepts an empty password submission.

## Repository

```text
crates/core/    decoder, image processing, affine matcher and NPZ templates
crates/driver/  Rust CLI, transport, enrollment, installer and VM management
crates/pam/     Rust PAM authentication module
pam/           explicit C authentication probes
kernel/        read-only BIOS-container helper and DKMS configuration
guest/         enclave ABI loader, entry assembly and guest bootstrap
tools/         offline vendor verification/export and release checks
tests/         synthetic parity tests and Python regression references
data/          matching policy and hardened systemd unit
docs/          installation, design, validation and troubleshooting
```

The repository contains source, build configuration, documentation and aggregate validation results. Captures, templates, keys, BIOS containers, vendor components and VM disks belong in private local storage. Public issues should contain sanitized hardware and software metadata.

## License and credits

User-space implementation: **GNU LGPL v3 or later**, SPDX `LGPL-3.0-or-later`. [License terms](LICENSE.md), [LGPL](COPYING.LESSER), [GPL terms referenced by the LGPL](COPYING), and [third-party notices](THIRD_PARTY_NOTICES.md) are included. The kernel helper's per-file license is documented in `LICENSE.md`.

Research references include the [GXFP51B7 EC discussion](https://github.com/PeshalaDilshan/OpenGoodixSPI/issues/16), [Sigfrodr/libfprint-goodixtls](https://github.com/Sigfrodr/libfprint-goodixtls), [Intel SGX driver](https://github.com/intel/linux-sgx-driver), and [QEMU SGX documentation](https://www.qemu.org/docs/master/system/i386/sgx.html). The matching policy is implemented with Rust array, filtering, FFT and interpolation libraries. The reference projects are credited in the third-party notices.
