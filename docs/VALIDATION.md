# Validation evidence

## Deployed v0.3 runtime — 2026-10-08

The MACHC-WAX9 deployment was upgraded from v0.2 to source revision `6efa076`
(v0.3.0). The installed helper, TOD adapter and PAM probes use the reviewed build.
The VM service now starts QEMU and checks guest readiness through the Rust
executable. Root-private recovery copies preserve the previous configuration,
binaries, calibration and enrollment.

The Rust VM service restarted successfully and remains enabled. An encrypted
empty-sensor capture passed integrity checks and decoded 5120 pixels from 10560
source bytes in approximately 1.29 seconds. Standard PAM accepted the existing
fprintd enrollment. Empty-sensor authentication returned `PAM_AUTHINFO_UNAVAIL`
(9). Cancelling a standard fprintd verification released the device, left zero
workers and permitted immediate enrollment enumeration.

The v0.3 calibration command stored a new 5120-pixel, twelve-bit background in
root-owned mode-0600 JSON and bound the local account. A new guided fprintd
right-index-finger enrollment completed all 12 accepted positions and persisted
a new root-private print. The administrator check observed:

| Fresh enrollment control | PAM result | Observed behavior |
| --- | ---: | --- |
| Empty sensor | 9 | Authentication unavailable |
| Enrolled index finger | 0 | Authentication and account checks succeeded |
| Different middle finger | 11 | One mismatch exhausted the configured attempt |
| Different ring finger | 11 | One mismatch exhausted the configured attempt |

The operator chose to conclude testing after the ring-finger control. The
little-finger control, completion of the five-case commissioning report and a
new SDDM-specific probe remain follow-up checks. The established standard
`pam_fprintd` SDDM branch was restored from the saved configuration, preserving
the password, account and session paths. The installed v0.3 runtime supplies the
current fingerprint path. Full desktop session startup is checked during a
normal user login.

Fifteen retired Python/custom-PAM runtime files and obsolete test services were
archived in the root-private recovery snapshot. The live deployment uses Rust
VM/capture commands, JSON calibration and standard fprintd/PAM. Signed guest
components and the existing pinned guest identity remain part of the runtime.

## Previous v0.2 ChicagoHS deployment — 2026-10-08

The deployed v0.2 implementation uses the Rust capture worker, ChicagoHS selector
207, libfprint TOD, official fprintd and standard `pam_fprintd`. The test machine
is a Huawei MACHC-WAX9 with BIOS 1.24, sensor firmware `0x2504`, host kernel
`7.2.9-arch1-1` and the pinned Ubuntu SGX guest. One local participant supplied
the enrollment and comparison captures.

The actual libfprint TOD adapter registered the EC sensor with official fprintd.
A new guided enrollment completed 12 accepted stages and persisted a private
right-index-finger print. Live fprintd verification accepted a fresh enrolled
finger and rejected a different finger. Empty-sensor verification ended with
an unavailable result. Cancellation released the device, left no worker process
and allowed the client to enumerate the enrollment immediately afterward.

After installing the persistent systemd and D-Bus activation configuration,
automatic daemon activation succeeded. An isolated standard `pam_fprintd`
authentication and `pam_unix` account check returned `PAM_SUCCESS` (0). Its
empty-sensor counterpart returned `PAM_AUTHINFO_UNAVAIL` (9). The service uses
its normal filesystem hardening with explicit ACPI doorbell and loopback access.
The marked SDDM branch now uses the staged official `pam_fprintd` module.
Authentication and account checks against the actual installed `sddm` PAM
service returned `PAM_SUCCESS` (0) with a fresh enrolled-finger press. The previous
SDDM configuration and daemon files are saved in private recovery storage.
Full desktop session startup remains a separate user login check.

## Source consolidation — v0.3

Version 0.3 preserves ChicagoHS, the decoder, device identity, 12-stage enrollment
and private fprintd print format. It consolidates administrator and installation
commands around standard PAM, stores new calibration as `background.json`, and
reads the background from an existing NPZ deployment. The previous matcher and
custom PAM implementation are available in Git history.

The source checks are recorded in [RELEASE_CHECKS.md](RELEASE_CHECKS.md). They
cover compilation, malformed input, worker event order and cancellation,
background compatibility, independent decoding/quality calculations, lint,
formatting, dependency auditing and release contents. The deployed v0.3 hardware run is recorded above. Fresh deployments use the
complete `gxfp51b7 check` sequence before administrator enablement. A clean-host
installation with a newly prepared private bundle requires a separate end-to-end
run; this machine exercised migration, calibration, enrollment and the completed
PAM controls.

## Matcher comparison — 2026-10-08

The comparison used the same 12 original enrollment presses for every backend,
the same background and the 12 independent probes listed above. Chicago's
position policy accepted eight stages from those old presses. The incomplete
offline gallery is comparison evidence; the live fprintd gallery was created
through a new, complete 12-stage enrollment.

| Backend | Same-finger evidence | Different-finger evidence | Mean scoring time |
| --- | --- | --- | --- |
| Affine NCC, 12 references | 6/6 accepted; 0.891410–0.922151 | 0/6 accepted; 0.382780–0.756304 | 2.337408 s |
| ChicagoHS, selector 207 | 6/6 accepted; scores 16–67 | 0/6 accepted; all scores −4 | 0.031864 s |
| OpenCV RootSIFT comparator | Correspondences in 4/6 probes; fused inliers 0–18 | Zero fused inliers in 6/6 probes | 0.008361 s |

RootSIFT provides numeric evidence with a separately calibrated policy still
required for authentication. The selected live matcher is ChicagoHS. The timing
measures matching on saved captures and excludes device acquisition and guest
startup. The sample set is one participant on one machine.


## Historical affine implementation

Before ChicagoHS integration, an affine NCC implementation used a fixed 0.86
threshold and a development gallery of 12 enrollment presses plus three early
same-finger probes. A fresh held-out set accepted six enrolled-index-finger
presses and rejected two presses each from the middle, ring and little fingers.
The Rust implementation reproduced the 12 Python decisions with a maximum
score difference of `4.44e-16`. This evidence belongs to the earlier algorithm.
The repository's current authentication path uses ChicagoHS.

## Scope of the evidence

Every source in the saved comparison passed the original component's integrity
check. Public records contain aggregate results; captures, templates, vendor
assets and guest files remain in private local storage. Matching times describe
saved captures and exclude acquisition and guest startup.

The measured sample covers one machine and one participant. Cross-person
acceptance, population false-accept/false-reject rates, liveness, spoof resistance,
other machines, suspend/resume, lockscreen and sudo require separate evaluation.
SDDM authentication/account success covers those PAM stages; desktop session
startup is checked through a normal interactive login.
