# Rust refactor checks — 2026-10-08

The Rust workspace provides the host executable, enrollment/administrator
commands, installer, VM lifecycle commands and PAM authentication library.
The v3 matching policy and existing NPZ templates carry over to this version.

## Offline verification

- Cargo workspace builds in release mode with the committed lockfile. The
  workspace also compiles with its minimum Rust version, 1.88.0.
- Twelve Rust unit tests cover device framing, malformed inputs, decoder layout,
  blank/invalid images, translation, affine bounds, template round trips,
  PAM configuration and subprocess deadlines.
- The 24 Python reference tests pass from `tests/reference`.
- Seven synthetic cross-language tests compare decoding, Gaussian filtering,
  quality statistics, affine samples/masks, translation, full 15-reference
  matching, threshold decisions and NPZ interoperability.
- A PAM ABI check covers six exported callbacks with null handles and invalid
  argument counts; these calls return a PAM error.
- Formatting, Clippy with warnings treated as errors, guest bootstrap syntax,
  local documentation links and the release inventory check pass.

One sequential synthetic 15-reference search took approximately 6.9 seconds in
Python and 2.1 seconds in Rust on the validated laptop. Rust reuses probe spectra
and the library's N-dimensional FFT processor. The timing describes that fixture
and machine; it is separate from biometric accuracy evidence.

## Hardware and template continuity

The Rust backend completed an encrypted live capture in approximately 1.9 seconds,
passed the original component's integrity checks and decoded 5120 pixels.
The pinned guest readiness check passed. An isolated Rust PAM check rejected an
empty sensor within the configured deadline.

The final Rust matcher was exercised against the existing private template and
all 12 independent v3 captures. All six enrolled-finger captures were accepted;
all six different-finger captures were rejected. Its maximum score difference
from the frozen Python results was `4.44e-16`. Mean offline Rust scoring time for
those captures was approximately 2.2 seconds.

The quantitative sample scope is documented in [VALIDATION.md](VALIDATION.md).
Generalized fresh-install enrollment and full desktop session startup are
separate deployment qualification steps. Public source contains synthetic
fixtures and aggregate results; local biometric and guest assets remain private.
