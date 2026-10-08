# GXFP51B7 Linux

[简体中文](README.zh-CN.md) · [Installation](docs/INSTALL.md) · [Design](docs/ARCHITECTURE.md) · [Validation](docs/VALIDATION.md)

Experimental Linux fingerprint acquisition and SDDM authentication for the **Huawei MACHC-WAX9** with ACPI device **GXFP51B7**.

The implementation was exercised on a real laptop: encrypted image capture, independent same-finger/different-finger comparisons, isolated PAM checks and authentication against the installed SDDM PAM service succeeded. The project provides a hardware-specific capture backend and PAM module for SDDM login. Its current release is experimental.

## Supported configuration

| Component | Validated configuration |
| --- | --- |
| Laptop / BIOS | Huawei MACHC-WAX9 / 1.24 |
| CPU | Intel Core i7-10510U, legacy SGX launch flow |
| Sensor / firmware | ChicagoHS `0x2504` / `GF_9ELIBE_EC_19024` |
| Host | EndeavourOS x86_64, kernel `7.2.9-arch1-1` |
| Login manager | SDDM 0.21, Arch `system-login` PAM layout |
| Isolated runtime | QEMU/KVM 11.1.2, Ubuntu 20.04, kernel `5.4.0-216-generic` |

The table defines the current hardware support and validation scope. Installation checks the machine identity and its ACPI resources against that configuration.

## How it works

An ACPI/EC mailbox carries the sensor's encrypted traffic. A small host module reads the BIOS-sealed communication container. Original signed Goodix and Intel enclaves run inside a same-machine KVM guest, unseal the communication key and validate the captured data. The host decodes an 80×64 image and compares it with a private local template. A bounded PAM helper supplies an optional fingerprint branch before the existing password path.

The communication key stays in the vendor enclave. Enrollment templates reside in a root-private directory; normal login capture returns image data in memory.

## Build and test

Install a C compiler, PAM development headers, Python 3.11 or newer, NumPy and SciPy, then:

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r requirements-tools.txt
make check PYTHON=python
```

These offline checks cover source compilation, packet parsing, image decoding, matching mathematics and PAM configuration generation. The guest loader and host kernel helper are separate build targets; see the [installation guide](docs/INSTALL.md).

## Installation and use

Installation combines this repository’s source with user-supplied vendor components, a prepared SGX guest and an SSH identity unique to that guest. The [guide](docs/INSTALL.md) documents preparation, host installation, enrollment, live validation, SDDM enablement and rollback. The installation workflow targets the validated machine and uses locally prepared assets.

After enrollment and successful live validation, select the enrolled account in SDDM, submit an empty password and touch the enrolled index finger. A mismatch, unavailable backend or timeout falls through to password authentication. Some SDDM themes may require a separate UI adjustment; the tested eos-breeze theme accepts an empty password submission.

## Results and limits

With a frozen threshold of 0.86 and one participant, fresh held-out captures produced **6/6 accepted same-finger presses** and **0/6 accepted presses from three other fingers**. These results describe a small, same-participant comparison. Population false-accept rate, liveness and spoof resistance require further evaluation.

The original deployment's actual SDDM PAM authentication and account checks returned success. Validation covered those authentication and account stages; full desktop session startup remains a follow-up check. The cleaned repository adds parameterized installation/enrollment tools with offline validation. Clean-machine hardware qualification is a next step. The new enrollment flow collects 15 fresh presses, while the original template combined 12 enrollment presses with 3 development presses. Each new template requires its own commissioning and biometric evaluation. See [validation details](docs/VALIDATION.md).

## Repository

```text
src/       host transport, image decoder, matcher and root-owned runtime
pam/       PAM module and explicit authentication probes
kernel/    read-only BIOS-container helper and DKMS configuration
guest/     SGX loader, entry assembly and isolated-guest bootstrap
tools/     vendor verification/export, installation and release inventory check
tests/     offline protocol, image, matching and PAM configuration tests
data/      matching policy and hardened systemd unit
docs/      reproduction, design, test evidence and troubleshooting
```

The repository contains source, build configuration, documentation and aggregate validation results. Captures, templates, keys, BIOS containers, vendor components and VM disks belong in private local storage. Public issues should contain sanitized hardware and software metadata.

## License and credits

User-space implementation: **GNU LGPL v3 or later**, SPDX `LGPL-3.0-or-later`. [License terms](LICENSE.md), [LGPL](COPYING.LESSER), [GPL terms referenced by the LGPL](COPYING), and [third-party notices](THIRD_PARTY_NOTICES.md) are included. The kernel helper's per-file license is documented in `LICENSE.md`.

Research references include the [GXFP51B7 EC discussion](https://github.com/PeshalaDilshan/OpenGoodixSPI/issues/16), [Sigfrodr/libfprint-goodixtls](https://github.com/Sigfrodr/libfprint-goodixtls), [Intel SGX driver](https://github.com/intel/linux-sgx-driver), and [QEMU SGX documentation](https://www.qemu.org/docs/master/system/i386/sgx.html). The current affine matcher is this project's implementation. The reference projects are credited in the third-party notices.
