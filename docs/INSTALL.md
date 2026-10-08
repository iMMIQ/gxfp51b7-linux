# Installation and recovery

This is an administrator/researcher workflow for the validated MACHC-WAX9 configuration. Read the complete sequence before changing authentication. Run the legacy Intel `isgx` driver inside the isolated guest, with the host’s current SGX manager handling physical EPC. Use the ACPI mailbox for sensor communication; `SPI0.0` is the laptop’s BIOS flash. Installation uses the existing sensor firmware and sealed key.

## 1. Host prerequisites

The tested host uses Arch/EndeavourOS with systemd, SDDM and the `system-login` PAM layout. Install Rust 1.99.0, a C toolchain, PAM and libfprint TOD development headers, GLib/GIO, pkg-config, official fprintd, kernel headers matching the running kernel, DKMS, `acpi_call`, QEMU with SGX/KVM support and OpenSSH. The isolated VM needs `/dev/kvm`, `/dev/sgx_vepc`, 48 MiB free EPC, approximately 1.5 GiB RAM and the `kvm`/`sgx` groups. BIOS SGX must be enabled.

The host runtime is the compiled Rust executable and libfprint TOD adapter. Standard fprintd and pam_fprintd provide authentication. Python, NumPy, pefile and cryptography support offline checks and asset preparation; Ruff and clang-format supply formatting/lint checks. Install `requirements-tools.txt` and `cargo-machete` in the development environment.

Run the offline checks first:

```sh
make check
```

## 2. Obtain and verify vendor components

Obtain Goodix package **1.1.151.18**, matching `ACPI\GXFP51B7`, from the [Microsoft Update Catalog](https://www.catalog.update.microsoft.com/Search.aspx?q=goodix%201.1.151.18) or your OEM. Obtain Intel Windows SGX PSW **2.18**, including `le.signed.dll`, `le_prod_css.bin` and `white_list_cert.bin`, through Intel/OEM distribution. Upstream release references: [Intel SGX 2.18](https://github.com/intel/linux-sgx/releases/tag/sgx_2.18). The validated launch flow uses that Windows enclave image. Obtain and use these packages under their Intel/OEM distribution terms.

Extract packages into private directories **outside this checkout**, then:

```sh
python tools/prepare_assets.py \
  --goodix /absolute/private/goodix-driver \
  --intel /absolute/private/intel-windows-psw \
  --output /absolute/private/fingerprint-bundle
```

The output directory must be new. The tool checks exact DLL SHA-256 hashes, signed enclave measurements and launch authorization, and extracts the volatile 256-byte ChicagoHS configuration from the verified DLL. The exporter accepts the exact validated package versions. Offline dependencies are in `requirements-tools.txt`. `assets.json` records the generated file digests; it records file integrity. Obtain the inputs from trusted Intel/OEM distribution sources.

## 3. Prepare your own isolated guest

Use an Ubuntu 20.04 guest with the tested `5.4.0-216-generic` kernel and matching headers. Pin that kernel during initial reproduction. Use Intel's [legacy SGX driver](https://github.com/intel/linux-sgx-driver) **2.11.0** inside the guest; obtain its source and retain its upstream notices. The tested source commit is `eea00a3f79bd1266adc7f4072c52a7a717b9e37e`; confirm `sgx_main.c` defines `DRV_VERSION` as `2.11.0` before building. The host's modern SGX manager remains in charge of physical EPC; QEMU presents virtual EPC to the guest.

Create a fresh Ed25519 key in your private bundle:

```sh
ssh-keygen -t ed25519 -N '' -C gxfp51b7-runtime \
  -f /absolute/private/fingerprint-bundle/id_ed25519
```

In the guest, provision account `ubuntu`, install its tools (`build-essential`, kernel headers, OpenSSH), and authorize **only this new public key** with the `restrict` prefix. `.ssh` must be owned by `ubuntu` with mode 0700 and `authorized_keys` owned by `ubuntu` with mode 0600. Bootstrap credentials and any cloud-init seed must be removed from the final guest.

Copy `guest/` source into the guest and place the legacy driver checkout at `/home/ubuntu/legacy-driver`. Build there:

```sh
cd /home/ubuntu/legacy-driver
make
cd /path/to/project/guest
make SGX_DRIVER=/home/ubuntu/legacy-driver
sudo install -o root -g root -m 0755 legacy_load /home/ubuntu/legacy_load
sudo bash bootstrap.sh
```

`bootstrap.sh` disables cloud-init, removes root authorized keys, restricts SSH to public-key authentication for `ubuntu`, and installs the guest-only module/read-only 9p mount service. Review it before running. The guest requires the normal Ubuntu administrator sudo configuration for `ubuntu`; the capture bridge runs `sudo timeout 12 /home/ubuntu/legacy_load ...` noninteractively. Configure that sudo access for `ubuntu` inside the dedicated guest.

Boot the guest on the same physical machine using the arguments in `crates/driver/src/runtime.rs`: KVM, `host,+sgx,+sgx-tokenkey,-sgxlc`, 48 MiB EPC and loopback TCP port 2228. For initial provisioning, use a private read-only 9p share arranged as:

```text
guest-share/
├── sgxs/
│   ├── le.signed.sgxs
│   ├── le.production.sigstruct
│   ├── WBDI_Enclave.signed.sgxs
│   └── WBDI_Enclave.signed.sigstruct
└── windows-psw218/
    └── white_list_cert.bin
```

Verify the SSH host key through the guest console, then create `known_hosts` in the private bundle with a pinned `[127.0.0.1]:2228` entry. Keep strict host-key checking enabled with the console-verified identity. Check guest service readiness, `/dev/isgx` and executable `/home/ubuntu/legacy_load`. Shut the guest down before preparing the final disk.

If your disk uses an external backing file, convert the stopped disk to a self-contained qcow2:

```sh
qemu-img convert -O qcow2 /absolute/private/source.qcow2 \
  /absolute/private/fingerprint-bundle/guest.qcow2
qemu-img info /absolute/private/fingerprint-bundle/guest.qcow2
```

The final bundle contains `assets.json`, its six vendor assets, `id_ed25519`, pinned `known_hosts` and `guest.qcow2`. Store the bundle in a private local directory outside the checkout. Stop provisioning VMs before starting the production VM; two 48 MiB guests exceed the tested machine's EPC capacity.

## 4. Install host helpers and runtime

Build/register the root-only BIOS reader with DKMS. Run from the repository root:

```sh
sudo install -d /usr/src/gxfp51b7-bios-0.1
sudo install -m 0644 kernel/goodix_bios_read.c kernel/Makefile kernel/dkms.conf \
  /usr/src/gxfp51b7-bios-0.1/
sudo dkms add -m gxfp51b7-bios -v 0.1
sudo dkms install -m gxfp51b7-bios -v 0.1
sudo modprobe acpi_call
sudo modprobe goodix_bios_read
```

With Secure Boot, follow your distribution's module-signing process. Configure these two helpers to load at boot using `/etc/modules-load.d/gxfp51b7.conf`; its contents are `acpi_call` and `goodix_bios_read`, one name per line.

Then install, substituting your own local account:

```sh
make
sudo build/gxfp51b7 install \
  --bundle /absolute/private/fingerprint-bundle --user YOUR_ACCOUNT
sudo systemctl enable --now gxfp51b7-vm.service
```

The Rust installer targets a fresh deployment using the tested Arch paths. It copies reviewed binaries into root-owned directories, creates the dedicated `gxfpvm` service account and installs a separate PAM test service. SDDM enablement is a later step after live validation. Existing deployments follow the [migration guide](MIGRATING.md). Installation writes files in stages; if a preparation error interrupts it, inspect and remove the newly installed project files before retrying.

## 5. Enroll and validate

```sh
sudo /usr/local/lib/gxfp51b7/gxfp51b7 calibrate --user YOUR_ACCOUNT
sudo systemctl restart fprintd.service
fprintd-enroll -f right-index-finger YOUR_ACCOUNT
sudo /usr/local/lib/gxfp51b7/gxfp51b7 check
```

Calibration captures an empty-sensor background into root-private `background.json`
and binds the local account. fprintd enrollment captures a fresh background and
12 accepted positions of the same finger. Keep contact through each capture,
lift fully when prompted, and vary contact position. fprintd stores the completed
print under `/var/lib/fprint`. Calibration takes place before enabling SDDM.

The live check asks for an empty sensor, the enrolled finger and three different
fingers. It checks standard PAM authentication and account results, and binds
its private report to the helper, adapter, PAM module, probes, background and
enrolled print digests. Expected PAM codes are 9 for an empty sensor, 0 for a
matching finger and 11 (`PAM_MAXTRIES`, one mismatch) for a different finger. Backend errors during a
specified different-finger control stop qualification. The isolated test
service uses the standard module and normal account checks; SDDM supplies
its own faillock integration after enablement.

## 6. Enable and test SDDM

```sh
sudo /usr/local/lib/gxfp51b7/gxfp51b7 enable
sudo /usr/local/lib/gxfp51b7/pam_sddm_probe YOUR_ACCOUNT
```

Enablement requires the previous live checks to pass for the current code and template, and accepts only the tested `auth include system-login` layout. It backs up the original SDDM file and preserves account, password and session configuration. The probe covers authentication and account checks. Test desktop session startup through a normal SDDM login.

On your next normal login, choose the enrolled account, submit an empty password and touch the enrolled index finger. The tested eos-breeze theme allows this. Password entry remains available after a mismatch/timeout. The configured authentication service is SDDM.

## Recovery and removal

To disable the fingerprint branch:

```sh
sudo /usr/local/lib/gxfp51b7/gxfp51b7 disable
```

It removes only the marked SDDM block, disables the fingerprint configuration and stops/disables the VM. Other PAM edits and the password path remain. The original file is stored at `/var/lib/gxfp51b7/backups/sddm-before-fingerprint`. To re-enable after disablement, start the VM again, calibrate, enroll with fprintd, rerun live checks, then enable.

For helper startup failures, use a root recovery console to inspect `/etc/pam.d/sddm` and remove only the `BEGIN GXFP51B7 fingerprint login` through `END GXFP51B7 fingerprint login` block. Preserve the rest of the file. After removing the block, stop the VM service. Remove the SDDM block before removing project authentication files.

For complete removal after disablement, remove the project's runtime/service, adapter/test-service/drop-in files and the two project state directories. Removing `/var/lib/gxfp51b7` erases calibration and commissioning data; use `fprintd-delete` for enrollment removal. Removing `/var/lib/gxfp51b7-vm` erases the guest. Complete project removal by removing its DKMS helper and modules-load entries. Keep shared packages, groups and `acpi_call` available for software that uses them.
