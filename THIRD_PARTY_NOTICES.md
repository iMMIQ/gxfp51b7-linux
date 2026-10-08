# Third-party references and required components

The following projects informed research or are externally obtained dependencies. Their licenses are not replaced by this repository's LGPL terms.

| Component | Role | Distribution here |
| --- | --- | --- |
| Goodix Windows 1.1.151.18 | GXFP51B7 protocol/configuration and original signed WBDI enclave | Not included; obtain through OEM/Microsoft under its own terms |
| Intel Windows SGX PSW 2.18 | Original signed launch enclave, production signature and whitelist | Not included; obtain under Intel's terms |
| Intel legacy SGX driver 2.11.0 | Guest-only `/dev/isgx` and ioctl ABI | Not included; obtain [upstream](https://github.com/intel/linux-sgx-driver) and retain its notices |
| Intel SGX 2.18 source | Public metadata/launch authorization ABI and public whitelist verification key | Public-key constants referenced in `tools/sgx_authorization.py`; SDK/runtime not bundled |
| QEMU/KVM, Linux, Linux-PAM, OpenSSH, systemd | Runtime infrastructure | External dependencies |
| NumPy, SciPy, pefile, cryptography | Image mathematics and offline PE/signature verification | External dependencies, not vendored |
| Sigfrodr/libfprint-goodixtls | Prior protocol/matcher research comparison | No SIFT source or binary included |
| OpenGoodixSPI issue #16 | GXFP51B7 EC protocol discussion | Link only |

The Intel driver advertises alternative BSD-3-Clause / GPL-v2 terms in its own `License.txt`; use the upstream distribution's actual notices when obtaining the header required by the guest build. `guest/legacy_load.c` includes `sgx_user.h` from that checkout, so it is not copied into this repository. The public authorization key is identified in [Intel's whitelist constants](https://github.com/intel/linux-sgx/blob/sgx_2.18/psw/ae/data/constants/linux/wl_pub.hh); it is a public verification key, not a private signing or sensor key.

The signed PE measurement reconstruction and host/guest protocol glue in this repository are independent implementations. Reverse-engineered facts and observed packet fixtures are documented; vendor decompilation output and copyrighted firmware/configuration bytes are not published. `tools/prepare_assets.py` extracts configuration locally only after verifying the supplied package.
