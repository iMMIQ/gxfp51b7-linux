# Local release preparation checks — 2026-10-08

The following checks were performed while preparing the initial source repository:

- `make check`: 24 offline tests passed, PAM module and both probes built with `-Wall -Wextra -Werror`, Python source compiled and guest bootstrap shell syntax checked.
- `make kernel`: read-only BIOS helper compiled against host kernel `7.2.9-arch1-1`; the check covered compilation.
- Guest loader and entry assembly compiled against the upstream legacy driver header; the check covered compilation.
- The new vendor asset exporter was exercised against the locally held validated vendor packages. Its enclave images, signatures and sensor configuration reproduced the original tested bytes. The exporter ran in a temporary private directory, which was removed after byte comparison.
- Image-decoder and affine-matcher source bodies were compared with the original validated versions; their code bodies match, with updated license notices.
- CLI help and local documentation links were checked.
- The release inventory rejected private/binary paths and scanned the tracked text files. The tracked release inventory consists of source, configuration, documentation and license texts.

Local Python was 3.14. The committed GitHub Actions workflow specifies Python
3.11–3.13; hosted job execution begins after GitHub publication. Generalized
installation/enrollment is the next clean-deployment qualification step. The
working research deployment retains its validated configuration. See [validation scope](VALIDATION.md).
