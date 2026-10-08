# Contributing

Run `make check` before submitting a change. Keep hardware-independent checks usable without root, vendor binaries or biometric samples. Describe the problem, changed behavior and validation performed. Distinguish compile/offline results from real hardware evidence.

Protocol changes need bounds and malformed-input coverage. Authentication changes must preserve the password fallback and account restrictions. Matcher changes need independent same-finger/different-finger evaluation; do not tune on data you later label held-out. Do not lower the threshold just to pass an enrollment.

Hardware support reports should identify model, BIOS, ACPI ID and software versions, with sanitized metadata. Do not send raw fingerprint images, processed templates, private keys, sealed BIOS data, vendor components, VM disks or decompiled vendor source.

Contributions use the per-file license shown in the source. Preserve third-party notices and identify any imported code and its license. This repository ships independently written host/guest glue, not extracted vendor binaries or an external matcher implementation.
