# Installation and recovery

This is an administrator/researcher workflow for the validated MACHC-WAX9 configuration. Read the complete sequence before changing authentication. Do not load the old Intel `isgx` driver on the host; it belongs only in the isolated guest. Do not access `SPI0.0`, which is BIOS flash on this laptop. No firmware update or persistent sensor key reset is needed.

## 1. Host prerequisites

The tested host uses Arch/EndeavourOS with systemd, SDDM and the `system-login` PAM layout. Install the system Python interpreter at `/usr/bin/python`, NumPy, SciPy, a C toolchain, PAM headers, kernel headers matching the running kernel, DKMS, `acpi_call`, QEMU with SGX/KVM support and OpenSSH. The isolated VM needs `/dev/kvm`, `/dev/sgx_vepc`, 48 MiB free EPC, approximately 1.5 GiB RAM and the `kvm`/`sgx` groups. BIOS SGX must be enabled.

The production helper uses isolated system Python (`python -I`); packages available only in a development virtual environment do not satisfy runtime dependencies.

Run the offline checks first:

```sh
make check
```

## 2. Obtain and verify vendor components

Obtain Goodix package **1.1.151.18**, matching `ACPI\GXFP51B7`, from the [Microsoft Update Catalog](https://www.catalog.update.microsoft.com/Search.aspx?q=goodix%201.1.151.18) or your OEM. Obtain Intel Windows SGX PSW **2.18**, including `le.signed.dll`, `le_prod_css.bin` and `white_list_cert.bin`, through Intel/OEM distribution. Upstream release references: [Intel SGX 2.18](https://github.com/intel/linux-sgx/releases/tag/sgx_2.18). A Linux launch enclave is not a substitute for the validated Windows image. This repository does not redistribute these packages or grant permission to redistribute them.

Extract packages into private directories **outside this checkout**, then:

```sh
python tools/prepare_assets.py \
  --goodix /absolute/private/goodix-driver \
  --intel /absolute/private/intel-windows-psw \
  --output /absolute/private/fingerprint-bundle
```

The output directory must be new. The tool checks exact DLL SHA-256 hashes, signed enclave measurements and launch authorization, and extracts the volatile 256-byte ChicagoHS configuration from the verified DLL. Unsupported versions are rejected. Offline dependencies are in `requirements-tools.txt`. `assets.json` records the generated file digests; it is an integrity record, not a substitute for trusting the input source.

## 3. Prepare your own isolated guest

Use an Ubuntu 20.04 guest with the tested `5.4.0-216-generic` kernel and matching headers. Pin that kernel during initial reproduction. Use Intel's [legacy SGX driver](https://github.com/intel/linux-sgx-driver) **2.11.0** inside the guest only; obtain its source and retain its upstream notices. The tested source commit is `eea00a3f79bd1266adc7f4072c52a7a717b9e37e`; confirm `sgx_main.c` defines `DRV_VERSION` as `2.11.0` before building. The host's modern SGX manager remains in charge of physical EPC; QEMU presents virtual EPC to the guest.

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

`bootstrap.sh` disables cloud-init, removes root authorized keys, restricts SSH to public-key authentication for `ubuntu`, and installs the guest-only module/read-only 9p mount service. Review it before running. The guest requires the normal Ubuntu administrator sudo configuration for `ubuntu`; the capture bridge runs `sudo timeout 12 /home/ubuntu/legacy_load ...` noninteractively. Configure that only in this dedicated guest, not on the host.

Boot the guest on the same physical machine using the arguments in `src/vm_start.py`: KVM, `host,+sgx,+sgx-tokenkey,-sgxlc`, 48 MiB EPC and loopback TCP port 2228. For initial provisioning, use a private read-only 9p share arranged as:

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

Verify the SSH host key through the guest console, then create `known_hosts` in the private bundle with a pinned `[127.0.0.1]:2228` entry. Do not trust an unverified network scan or disable host-key checking. Check guest service readiness, `/dev/isgx` and executable `/home/ubuntu/legacy_load`. Shut the guest down before preparing the final disk.

If your disk uses an external backing file, convert the stopped disk to a self-contained qcow2:

```sh
qemu-img convert -O qcow2 /absolute/private/source.qcow2 \
  /absolute/private/fingerprint-bundle/guest.qcow2
qemu-img info /absolute/private/fingerprint-bundle/guest.qcow2
```

The final bundle contains `assets.json`, its six vendor assets, `id_ed25519`, pinned `known_hosts` and `guest.qcow2`. Keep it private and never commit it. Stop provisioning VMs before starting the production VM; two 48 MiB guests exceed the tested machine's EPC capacity.

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
sudo /usr/bin/python tools/install.py \
  --bundle /absolute/private/fingerprint-bundle --user YOUR_ACCOUNT
sudo systemctl enable --now gxfp51b7-vm.service
```

The installer targets the tested Arch paths, copies reviewed code into root-owned directories, creates the dedicated `gxfpvm` service account and installs a separate PAM test service. It refuses existing deployments and does not modify SDDM. There is no in-place migration from the original research runtime; do not run this installer over that runtime. Installation is not a transaction: if a preparation error interrupts it, inspect and remove only the newly installed project files before retrying.

## 5. Enroll and validate

```sh
sudo /usr/bin/python -I /usr/local/lib/gxfp51b7/manage.py enroll --user YOUR_ACCOUNT
sudo /usr/bin/python -I /usr/local/lib/gxfp51b7/manage.py check
```

Enrollment captures an empty background and 15 separate presses of the same index finger. Lift the finger fully between presses and vary position slightly. No raw capture is saved; the processed template is root-private. Changing an enrollment requires disabling SDDM fingerprint login first.

The live check asks for an empty sensor, the enrolled finger and three different fingers. It records expected outcomes privately and binds that report to the runtime, PAM module and template digests. Errors or unexpected acceptances stop the sequence. These checks are a local commissioning aid, not a population security evaluation. Collect independent repetitions before routine use.

## 6. Enable and test SDDM

```sh
sudo /usr/bin/python -I /usr/local/lib/gxfp51b7/manage.py enable
sudo /usr/local/lib/gxfp51b7/pam_sddm_probe YOUR_ACCOUNT
```

Enablement requires the previous live checks to pass for the current code and template, and accepts only the tested `auth include system-login` layout. It backs up the original SDDM file and preserves account, password and session configuration. The probe performs authentication and account checks only; it does not open a desktop session.

On your next normal login, choose the enrolled account, submit an empty password and touch the enrolled index finger. The tested eos-breeze theme allows this. Password entry remains available after a mismatch/timeout. This release does not configure screen lock or sudo.

## Recovery and removal

To disable the fingerprint branch:

```sh
sudo /usr/bin/python -I /usr/local/lib/gxfp51b7/manage.py disable
```

It removes only the marked SDDM block, disables the fingerprint configuration and stops/disables the VM. Other PAM edits and the password path remain. The original file is stored at `/var/lib/gxfp51b7/backups/sddm-before-fingerprint`. To re-enable after disablement, start the VM again, enroll, rerun live checks, then enable.

If the helper cannot run, use a root recovery console to inspect `/etc/pam.d/sddm` and remove only the `BEGIN GXFP51B7 fingerprint login` through `END GXFP51B7 fingerprint login` block. Preserve the rest of the file. After removing the block, stop the VM service. Do not delete the PAM module while its SDDM block still refers to it.

For complete removal after disablement, remove the project's runtime/service, PAM module/test-service files and the two project state directories. Removing `/var/lib/gxfp51b7` erases the enrollment template; removing `/var/lib/gxfp51b7-vm` erases the guest. Remove the DKMS helper and its modules-load entries if no longer needed. Do not remove shared packages, groups or `acpi_call` that other software uses.
