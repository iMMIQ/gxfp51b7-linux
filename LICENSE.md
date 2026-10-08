# License scope

This project's original
code is licensed under the **GNU Lesser General Public License, version 3 or
any later version**, SPDX `LGPL-3.0-or-later`.

You may redistribute and modify that code under those terms. The applicable warranty and liability terms are included in the full license
texts. See [COPYING.LESSER](COPYING.LESSER) and
[COPYING](COPYING) for the complete terms. The LGPL incorporates portions of
GPL v3; the user-space source carries the LGPL-3.0-or-later grant.

The original code under `kernel/` is offered under your choice of
**GPL-2.0-only OR LGPL-3.0-or-later**. Choose its GPL v2 option when building and
using it as a Linux kernel module. Linux's kernel code requires GPL-v2-compatible
licensing; the source SPDX expression and `MODULE_LICENSE("GPL")` consistently
identify that permitted kernel use. See [kernel/COPYING](kernel/COPYING) and the
[kernel licensing rules](https://docs.kernel.org/process/license-rules.html).

Documentation and project-authored build/configuration files follow
LGPL-3.0-or-later unless otherwise marked. The authoritative expression for an
individually marked file is its SPDX notice. External dependencies and vendor
components retain their own licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

The vendored ChicagoHS algorithm and tests under `crates/backends/native/chicago`,
`common` and `tests` retain **LGPL-2.1-or-later** and their original copyright notices.
Their complete license is in [crates/backends/native/COPYING.LESSER](crates/backends/native/COPYING.LESSER).
The Rust wrapper, bridge and libfprint adapter are project-authored LGPL-3.0-or-later code.

The licensing grant covers the project-authored source and documentation.
Vendor enclaves, driver DLLs, sensor configuration, firmware and dependencies
are obtained under their respective distribution terms. Keys, templates and
guest disks are deployment-specific private assets.
