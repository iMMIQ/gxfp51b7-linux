# ChicagoHS source provenance

Source: https://github.com/berkekbgz/libfprint-goodix-spi

Pinned revision: `010a665f54089a1632b1ab7be588b316ace934e2`.

The ChicagoHS algorithm, CRC primitive and synthetic native tests are copied
verbatim from that revision. Copyright and LGPL-2.1-or-later notices remain
in each file. The upstream LGPL text is in COPYING.LESSER.

This source implements an I/O-free algorithm. Device transport, credentials,
firmware, original vendor programs and private fixtures belong to the local
installation. Our bridge provides a bounded ABI used by Rust.

`SHA256SUMS` records every copied source, test and upstream license.
`tools/test_native.py` checks these hashes before running the synthetic suite.
Project-authored `bridge.c` is maintained separately under LGPL-3.0-or-later.
