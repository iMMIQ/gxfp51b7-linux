# Validation evidence

## Original hardware deployment — 2026-10-08

One MACHC-WAX9 laptop and one local participant were used. The v3 algorithm and threshold were frozen before the following fresh, independent capture set:

| Capture group | Attempts | Accepted | Correlation range |
| --- | ---: | ---: | --- |
| Enrolled index finger | 6 | 6 | 0.891410–0.922151 |
| Different middle finger | 2 | 0 | 0.462673–0.560500 |
| Different ring finger | 2 | 0 | 0.739430–0.756304 |
| Different little finger | 2 | 0 | 0.523049–0.628425 |

Every listed source passed the original component's CRC check. The threshold was 0.86. All different-finger controls were from the same participant. These results are not a cross-person impostor study, a general false-accept rate, or a liveness/spoof-resistance claim.

The frozen reference set contained 12 enrollment presses plus 3 same-finger presses from the first failed verification experiment. Those added presses were development/enrollment data and were excluded from the fresh v3 evaluation. Prior v2 probes informed v3 development and were also excluded from independent v3 evidence. The public repository includes aggregated results only, without raw images, per-capture private identifiers or templates.

The original protected runtime completed an in-memory capture in approximately 1.9 seconds. Its service was restarted and became ready again. The current-kernel BIOS helper was rebuilt, installed and reloaded through DKMS. Independent PAM checks rejected an empty sensor, accepted the enrolled finger and rejected a different finger. Authentication and account checks against the actual installed `sddm` PAM service returned `PAM_SUCCESS` (0).

A full logout/reboot followed by desktop session opening was not performed during that test. Desktop lockscreen, sudo, other hardware, newer guest kernels, cross-person controls, spoof attempts and suspend/resume were not qualified.

## Cleaned repository

The current affine matcher and image decoder retain the validated algorithm. License headers, deployment paths, prompts, account parameters and administrator tooling were cleaned for publication. New tooling avoids relying on a participant's private research directory and does not reuse the original participant's template.

The public enrollment command collects **15 new separate presses**. The original quantitative biometric evidence concerns the original 12+3 template. A new 15-press template and the generalized installation/validation commands have not received a clean-machine end-to-end hardware qualification. Commissioning is required on each deployment. The cleanup does not overwrite the working research runtime.

Offline tests cover packet checksums and malformed lengths, image nibble order/column layout/padding, bounded translation and deformation, blank/nonfinite images, unrelated synthetic texture, and PAM block insertion/removal with account-name injection and unsupported-layout rejection. Synthetic matcher tests detect mathematical regressions; they are not biometric security validation.

CI builds the PAM module/probes and runs offline checks on Python 3.11–3.13. Kernel and guest sources can be compiled locally without loading or running them. CI does not access real hardware or mutate PAM.
