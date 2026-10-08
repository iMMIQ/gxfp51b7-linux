# Quality and release checks — 2026-10-08

## Toolchain and automated checks

Version 0.3 pins Rust 1.99.0 with rustfmt and Clippy. The local build uses GCC 16
for host/guest C and the running kernel's build system for the BIOS helper.
Ruff 0.16.10 and clang-format 23.1.2 are pinned in the development requirements.
CI runs the same `make check` and `make audit` targets on Ubuntu 24.04.

The check target covers:

- Locked release builds of the Rust workspace, libfprint TOD adapter and PAM/API
  probes; C builds use `-Wall -Wextra -Werror`.
- Ten Rust tests for device framing, decoding/quality, legacy background loading,
  Chicago context/input checks, print bounds/version handling, administrator PAM
  layout and subprocess deadlines.
- Three GLib worker test groups exercising the actual adapter pipe reader with
  valid results, malformed events, invalid event order, NUL bytes, bounded print
  lengths, complete/truncated enrollment payloads, incomplete input and cancellation.
- Seven pinned ChicagoHS test programs with 103 registered cases: 61 synthetic
  cases pass and 42 optional private-oracle cases skip. Hashes cover all 29 pinned
  upstream source files.
- Eleven independent Python decoder/protocol tests and two Rust/Python parity
  cases for decoding and quality statistics.
- Rust/C/Python formatting, strict Clippy, Ruff, unused direct dependencies,
  Rust documentation with warnings treated as errors, Python compilation,
  guest bootstrap syntax and the public release inventory.

Local platform checks additionally compile the BIOS helper against host kernel
`7.2.9-arch1-1` and the guest loader against legacy SGX 2.11.0 headers. Clang's
static analyzer reports zero warnings for the first-party TOD adapter.
`make audit` reports zero known vulnerabilities and zero warnings against RustSec.
All listed local checks passed; the public release inventory contains 107 text
files and passes its private/binary-content checks.

## Code and dependency cleanup

The workspace has three packages: core decoding/quality, the Chicago wrapper and
the host driver. The removed affine matcher, RootSIFT comparator, custom PAM
crate and research CLI are preserved in Git history. Resolved Cargo packages
fell from 158 to 91, including the three workspace packages. OpenCV, FFT,
affine interpolation/filtering and custom PAM dependencies were removed.
`cargo machete` reports zero unused direct dependencies.

Application-private exports use crate visibility. Unsafe operations require
explicit blocks and safety comments. Worker input rejects NUL bytes and bounds
lines/print data. Subprocess deadline handling terminates the process group
while its leader remains unreaped, preserving the leader's identity through
cleanup. Standard PAM qualification distinguishes authentication rejection
from an unavailable backend and binds the selected module, print and binaries
to the stored commissioning evidence.

Merged development branches `rust-refactor` and `chicago-validation` were
removed locally and from GitHub. The cleanup branch is retained for review.
The release inventory contains source, synthetic tests and documentation;
biometric captures, guest disks, keys and vendor binaries remain private.

## Quality assessment and remaining qualification

The active code has one authentication path, reproducible tooling and a shared
local/CI quality gate. Source consolidation removes parallel matcher policies,
duplicate Python administration and stale runtime artifacts. Kernel, guest and
vendor algorithm boundaries remain explicit and documented.

The strongest remaining limits concern deployment and biometric evidence. The
live ChicagoHS evidence covers one machine and one participant, and the 42
private-oracle native cases require external fixtures. The revised v0.3 fresh
installation, calibration and commissioning sequence needs a complete hardware
run before deployment. Its offline tests establish parser/format compatibility
and software behavior. The hardware-qualified v0.2 installation continues to
provide the current login path during this source cleanup.

[VALIDATION.md](VALIDATION.md) records the measured comparison, live fprintd/PAM
results and the scope of the biometric evidence. General false-accept rates,
spoof resistance and full desktop session startup require their own checks.
