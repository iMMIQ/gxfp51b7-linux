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

## Cleaned repository

The current affine matcher and image decoder retain the validated algorithm. License headers, deployment paths, prompts, account parameters and administrator tooling were cleaned for publication. The new tooling uses explicit account and private-bundle parameters and generates a local template during enrollment.

The public enrollment command collects **15 new separate presses**. The original quantitative biometric evidence concerns the original 12+3 template. Clean-machine end-to-end hardware qualification is the next step for the generalized installation/validation commands and new 15-press templates. Each deployment requires its own commissioning. Repository preparation uses a separate source directory; the working research runtime retains its validated configuration.

Offline tests cover packet checksums and malformed lengths, image nibble order/column layout/padding, bounded translation and deformation, blank/nonfinite images, unrelated synthetic texture, and PAM block insertion/removal with account-name injection and unsupported-layout rejection. Synthetic matcher tests detect mathematical regressions. Biometric security validation uses independent captures and participant controls.

CI builds the PAM module/probes and runs offline checks on Python 3.11–3.13. Kernel and guest compilation are separate local checks. CI operates on source, synthetic fixtures and generated configuration text.
