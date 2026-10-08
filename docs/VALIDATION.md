# Validation evidence

## Original hardware deployment — 2026-10-08

One MACHC-WAX9 laptop and one local participant were used. The v3 algorithm and threshold were frozen before the following fresh, independent capture set:

| Capture group | Attempts | Accepted | Correlation range |
| --- | ---: | ---: | --- |
| Enrolled index finger | 6 | 6 | 0.891410–0.922151 |
| Different middle finger | 2 | 0 | 0.462673–0.560500 |
| Different ring finger | 2 | 0 | 0.739430–0.756304 |
| Different little finger | 2 | 0 | 0.523049–0.628425 |

Every listed source passed the original component's CRC check. The threshold was 0.86. All different-finger controls were from the same participant. The evidence covers same-participant finger comparisons. Cross-person acceptance, general false-accept rate, liveness and spoof resistance remain evaluation tasks.

The frozen reference set contained 12 enrollment presses plus 3 same-finger presses from the first failed verification experiment. Those added presses and prior v2 probes belong to the development dataset. The independent v3 evaluation used the fresh capture set shown above. Public validation records contain aggregate results; images and templates reside in private local storage.

The original protected runtime completed an in-memory capture in approximately 1.9 seconds. Its service was restarted and became ready again. The current-kernel BIOS helper was rebuilt, installed and reloaded through DKMS. Independent PAM checks rejected an empty sensor, accepted the enrolled finger and rejected a different finger. Authentication and account checks against the actual installed `sddm` PAM service returned `PAM_SUCCESS` (0).

The completed login checks cover SDDM authentication and account stages. Follow-up qualification covers full desktop session startup, desktop lockscreen, sudo, other hardware, newer guest kernels, cross-person controls, spoof attempts and suspend/resume.

## Rust implementation — 2026-10-08

The final Rust implementation reproduces the current v3 policy with library-based
Gaussian filtering, FFT correlation, affine interpolation and NPZ I/O. It accepts
the existing enrollment without converting or re-enrolling it. Across the same
12 independent private captures, every acceptance/rejection matched Python and
the maximum score difference was `4.44e-16`. These comparisons establish
implementation continuity on that dataset.

The Rust transport completed a live encrypted capture, verified the enclave's
integrity result and decoded 5120 pixels. An isolated Rust PAM test rejected an
empty sensor. A fresh enrolled-finger verification through the Rust executable
passed the quality gates and scored `0.861181` against the frozen `0.86` threshold.
The subsequent PAM diagnostic captured an empty sensor throughout its presence
window and rejected it before matching. Enrolled-finger PAM qualification is
pending a fresh press held through the same invocation.
See [release checks](RELEASE_CHECKS.md) for build and parity evidence.

## Cleaned repository

The current affine matcher and image decoder retain the validated algorithm. License headers, deployment paths, prompts, account parameters and administrator tooling were cleaned for publication. The new tooling uses explicit account and private-bundle parameters and generates a local template during enrollment.

The public enrollment command collects **15 new separate presses**. The original quantitative biometric evidence concerns the original 12+3 template. Clean-machine end-to-end hardware qualification is the next step for the generalized installation/validation commands and new 15-press templates. Each deployment requires its own commissioning. Repository preparation uses a separate source directory; the working research runtime retains its validated configuration.

Offline tests cover packet checksums and malformed lengths, image nibble order/column layout/padding, bounded translation and deformation, blank/nonfinite images, unrelated synthetic texture, and PAM block insertion/removal with account-name injection and unsupported-layout rejection. Synthetic matcher tests detect mathematical regressions. Biometric security validation uses independent captures and participant controls.

CI builds the Rust workspace and C probes, checks formatting/Clippy, and runs Rust unit tests, Python reference tests and synthetic cross-language comparisons. Kernel and guest compilation are separate local checks. CI operates on source, synthetic fixtures and generated configuration text.

## ChicagoHS and standard integration — 2026-10-08

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
