# Local release preparation checks — 2026-10-08

The following checks were performed while preparing the initial source repository:

- `make check`: 24 offline tests passed, PAM module and both probes built with `-Wall -Wextra -Werror`, Python source compiled and guest bootstrap shell syntax checked.
- `make kernel`: read-only BIOS helper compiled against host kernel `7.2.9-arch1-1`; no module was loaded during repository preparation.
- Guest loader and entry assembly compiled against the upstream legacy driver header; the newly built guest executable was not run on the host.
- The new vendor asset exporter was exercised against the locally held validated vendor packages. Its enclave images, signatures and sensor configuration reproduced the original tested bytes. Temporary extracted vendor files were removed and were never staged in Git.
- Image-decoder and affine-matcher source bodies were compared with the original validated versions; only their license notices changed.
- CLI help and local documentation links were checked.
- The release inventory rejected private/binary paths and scanned the tracked text files. Build products, biometric material, vendor components, SSH identities and guest disks are not tracked.

Local Python was 3.14. The committed GitHub Actions workflow specifies Python
3.11–3.13; those hosted jobs have not run before publication. Generalized
installation/enrollment has not been executed over the working research
deployment. See [validation scope](VALIDATION.md).
