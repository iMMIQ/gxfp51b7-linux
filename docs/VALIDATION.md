# Validation evidence

## Deployed ChicagoHS login — 2026-10-08

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
formatting, dependency auditing and release contents. The currently installed
v0.2 login remains the hardware-qualified deployment. A v0.3 deployment requires
its own live commissioning with `gxfp51b7 check` before enablement; the revised
fresh-install/calibration workflow requires an end-to-end hardware run.

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
