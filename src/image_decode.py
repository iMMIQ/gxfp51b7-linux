#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""ChicagoHS 80x64 image decoding, independently matching the vendor regroup.
Input is the 10560-byte source exported by WBDI ecall 0x15, or a
compact 7680-byte pixel payload. Actual EC captures store each column's
96 packed bytes in a 132-byte slot with 36 zero padding bytes.
This module does not access hardware or perform authentication.
"""
import struct
WIDTH, HEIGHT = 80, 64
PACKED_LENGTH = WIDTH * HEIGHT * 3 // 2


def decode(source):
    if len(source) == 10560:
        columns = [source[i:i+132] for i in range(0, 10560, 132)]
        if any(any(column[96:]) for column in columns):
            raise ValueError('Unexpected ChicagoHS column padding')
        source = b''.join(column[:96] for column in columns)
    elif len(source) != PACKED_LENGTH:
        raise ValueError('Unexpected ChicagoHS image length')
    pixels = [0] * (WIDTH * HEIGHT)
    for off in range(0, PACKED_LENGTH, 6):
        a, b, c, d, e, f = source[off:off+6]
        values = ((a & 15) * 256 + b, d * 16 + (a >> 4),
                  c + (f & 15) * 256, e * 16 + (f >> 4))
        for j, value in enumerate(values):
            k = off // 6 * 4 + j
            pixels[(k & 63) * WIDTH + (k >> 6)] = value
    return pixels


def pgm(source):
    pixels = decode(source)
    return b'P5\n80 64\n4095\n' + struct.pack('>5120H', *pixels)
