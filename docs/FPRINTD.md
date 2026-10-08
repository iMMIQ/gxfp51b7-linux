# ChicagoHS and standard fingerprint login

The GXFP51B7 adapter connects the Rust encrypted acquisition worker and the
ChicagoHS matcher to libfprint TOD. fprintd provides enrollment storage, D-Bus
access control and the standard `pam_fprintd` login module. The device uses
12 accepted enrollment positions. Its feedback asks for a complete lift and
additional coverage as enrollment progresses.

## Build

Install GLib/GIO development headers, OpenCV 4 or 5 and its development headers,
libclang, pkg-config, Linux-PAM development headers and libfprint TOD development
headers. Rust 1.88 or newer builds the host workspace.

```sh
make check fprint
```

On Ubuntu 24.04 the development packages are `libglib2.0-dev`, `libopencv-dev`,
`libclang-dev`, `pkg-config`, `libpam0g-dev` and `libfprint-2-tod-dev`.
The TOD driver is installed into the library's `tod-1` directory. The following
paths correspond to Arch's libfprint-tod package:

```sh
sudo install -m 0755 build/gxfp51b7 /usr/local/lib/gxfp51b7/gxfp51b7
sudo install -Dm0644 build/libfprint-gxfp51b7.so /usr/lib/libfprint-2/tod-1/libfprint-gxfp51b7.so
sudo install -Dm0644 data/fprintd-gxfp51b7.conf /etc/systemd/system/fprintd.service.d/gxfp51b7.conf
sudo systemctl daemon-reload
sudo systemctl restart fprintd.service
```

The drop-in enables the real EC device through a fixed discovery value and
permits the worker's loopback SSH connection and ACPI doorbell write. The host
module, ACPI resources, SGX guest, private assets and account commissioning are
prepared with the [installation guide](INSTALL.md). The account configuration
must be enabled and bound to the intended regular local account. The existing
private NPZ background supplies the initial finger-off calibration gate; the
new enrollment obtains its own fresh background and Chicago gallery.

## Enrollment and validation

Start with the sensor clear. Then run:

```sh
fprintd-enroll -f right-index-finger YOUR_ACCOUNT
fprintd-list YOUR_ACCOUNT
fprintd-verify -f right-index-finger YOUR_ACCOUNT
```

Use the same finger for all 12 accepted positions. Keep it in contact during a
capture, lift completely when requested, then vary contact position slightly.
Repeated positions can generate a centering retry; an accepted position advances
the stage counter. fprintd stores the completed private print under
`/var/lib/fprint`. Enrollment ends after 50 attempts or 10 minutes. A capture has
its own bounded timeout. Cancellation closes the worker and its subprocess group.

Check a fresh enrolled-finger press, a different finger, an empty sensor and
cancellation before changing the login configuration. The public libfprint probe
supports a separate private print file for API qualification:

```sh
sudo env FP_GXFP51B7_DEVICE=/dev/goodix_bios_sealed build/fprint_probe list
sudo env FP_GXFP51B7_DEVICE=/dev/goodix_bios_sealed build/fprint_probe enroll YOUR_ACCOUNT /absolute/private/print
sudo env FP_GXFP51B7_DEVICE=/dev/goodix_bios_sealed build/fprint_probe verify YOUR_ACCOUNT /absolute/private/print
```

The probe uses libfprint directly. Run each hardware operation separately so
fprintd and the direct probe have exclusive access during their respective tests.

## SDDM

Use a separate PAM test service with `pam_fprintd.so max-tries=1 timeout=20`
and the system's normal account checks. After same-finger authentication and
account checks pass, replace the module line inside the marked SDDM branch with:

```text
auth [success=ok default=1] pam_fprintd.so max-tries=1 timeout=20
```

The branch's shell, nologin, environment and faillock checks remain part of the
login flow. Its following `pam_faillock authsucc` line handles success; the
original `system-login` include supplies the password fallback. Keep a private
copy of the previous SDDM file for restoration. SDDM reads PAM configuration
for each new authentication attempt.

The `gxfp51b7 enable` and `check` commands commission the custom affine PAM path.
For the standard fprintd path, use the separate PAM checks described here and
preserve its own deployment evidence. `gxfp51b7 disable` removes the project's
marked SDDM branch and stops the acquisition VM.

## Existing open-fprintd installations

One daemon owns `net.reactivated.Fprint` at a time. Choose official fprintd as
the owner for this adapter. Save the existing service configuration, stop
open-fprintd, and configure the official daemon with the drop-in above. Restore
the previous daemon configuration when rolling back.

The qualified laptop retained its installed open-fprintd packages and clients.
A package-manager-verified official Arch fprintd daemon and PAM module were
staged under `/usr/local/lib/gxfp51b7`; a local `ExecStart` override selects that
daemon, and open-fprintd is masked. This local arrangement is reversible by
removing the local systemd and `/usr/local/share/dbus-1/system-services/net.reactivated.Fprint.service` activation overrides, reloading D-Bus configuration, unmasking open-fprintd and restoring the saved PAM
file. Fresh deployments can use their distribution's official fprintd package.

## Offline matcher comparison

The Rust `compare` command evaluates the same frozen references and independent
probes with affine NCC, ChicagoHS and an OpenCV RootSIFT comparator:

```sh
build/gxfp51b7 compare --manifest /absolute/private/dataset.json
```

The manifest contains `background`, `references` and `probes`. Each probe has a
`name`, `source` and `enrolled` boolean. Capture paths resolve relative to the
manifest directory. Output contains aggregate scores, quality, enrollment
feedback and timing. Keep the manifest, captures and generated output in private
storage. RootSIFT is a research comparator whose numeric evidence requires its
own independently calibrated decision policy. Chicago uses the pinned upstream
selector and match rule; verification evaluates one quality-accepted capture.
