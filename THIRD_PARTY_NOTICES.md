# Third-party references and required components

The following projects informed research or are externally obtained dependencies. Each component retains its own licensing terms.

| Component | Role | Distribution here |
| --- | --- | --- |
| Goodix Windows 1.1.151.18 | GXFP51B7 protocol/configuration and original signed WBDI enclave | User-supplied through OEM/Microsoft distribution |
| Intel Windows SGX PSW 2.18 | Original signed launch enclave, production signature and whitelist | User-supplied under Intel's distribution terms |
| Intel legacy SGX driver 2.11.0 | Guest-only `/dev/isgx` and ioctl ABI | User-supplied [upstream](https://github.com/intel/linux-sgx-driver) source with its notices |
| Intel SGX 2.18 source | Public metadata/launch authorization ABI and public whitelist verification key | Public-key constants referenced in `tools/sgx_authorization.py`; SDK/runtime obtained externally |
| QEMU/KVM, Linux, Linux-PAM, OpenSSH, systemd | Runtime infrastructure | External dependencies |
| NumPy, SciPy, pefile, cryptography | Offline regression reference and PE/signature verification | Development dependencies |
| [berkekbgz/libfprint-goodix-spi](https://github.com/berkekbgz/libfprint-goodix-spi) | ChicagoHS algorithm, CRC primitive and synthetic tests; Berke Kabagöz, LGPL-2.1-or-later | Verbatim pinned source; [provenance](crates/backends/native/UPSTREAM.md) |
| [goodix-fp-linux-dev/sigfm](https://github.com/goodix-fp-linux-dev/sigfm) | Feature matching research | Link only; our RootSIFT comparator uses OpenCV |
| OpenCV, GLib/GIO, libfprint TOD, fprintd | Matching library and standard fingerprint infrastructure | External dependencies |
| Sigfrodr/libfprint-goodixtls | Prior protocol/matcher research comparison | Research reference |
| OpenGoodixSPI issue #16 | GXFP51B7 EC protocol discussion | Link only |

The Intel driver advertises alternative BSD-3-Clause / GPL-v2 terms in its own `License.txt`; use the upstream distribution's actual notices when obtaining the header required by the guest build. `guest/legacy_load.c` includes `sgx_user.h` from that checkout, as an external build dependency. The public authorization key is identified in [Intel's whitelist constants](https://github.com/intel/linux-sgx/blob/sgx_2.18/psw/ae/data/constants/linux/wl_pub.hh); it verifies the public launch authorization chain.

The signed PE measurement reconstruction and host/guest protocol glue in this repository are independent implementations. The repository documents reverse-engineered facts and observed packet fixtures. Vendor configuration is extracted locally from the user-supplied package. `tools/prepare_assets.py` extracts configuration locally only after verifying the supplied package.

## Rust dependencies

`Cargo.lock` records the complete resolved dependency graph. The [full inventory](THIRD_PARTY_RUST.md) lists resolved versions and upstream license expressions. Library source and
license notices are available through Cargo's registry packages. The direct
libraries retain these upstream terms:

| Crates | SPDX license |
| --- | --- |
| `ndarray`, `ndarray-conv`, `ndarray-ndimage`, `ndarray-npy`, `interpn` | MIT OR Apache-2.0 |
| `anyhow`, `clap`, `serde`, `serde_json`, `libc`, `memmap2`, `sha2`, `tempfile`, `wait-timeout` | MIT OR Apache-2.0 |
| `nix`, `pam-bindings`, `opencv` | MIT |
| `cc`, `pkg-config` | MIT OR Apache-2.0 |
| `rustix` | Apache-2.0 OR Apache-2.0 WITH LLVM-exception OR MIT |

The application's own source uses LGPL-3.0-or-later. Vendor components and the
externally installed guest SGX driver retain their respective distribution terms.
