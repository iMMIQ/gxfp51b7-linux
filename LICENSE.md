# License scope

Except for the separately identified kernel helper, this project's original
code is licensed under the **GNU Lesser General Public License, version 3 or
any later version**, SPDX `LGPL-3.0-or-later`.

You may redistribute and modify that code under those terms. It is distributed
without any warranty, including implied warranties of merchantability or
fitness for a particular purpose. See [COPYING.LESSER](COPYING.LESSER) and
[COPYING](COPYING) for the complete terms. The LGPL incorporates portions of
GPL v3; including GPL v3's text does not change the user-space code to GPL-only.

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

No vendor enclave, driver DLL, sensor configuration binary, firmware, VM disk,
private key or biometric template is licensed or distributed by this repository.
