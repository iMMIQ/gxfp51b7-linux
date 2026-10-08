# SPDX-License-Identifier: LGPL-3.0-or-later
"""Independent packet parsing reference used by synthetic tests."""

import struct


def encode(command, payload=b""):
    inner = bytes([command]) + struct.pack("<H", len(payload) + 1) + payload
    body = inner + bytes([(0xAA - sum(inner)) & 255])
    prefix = b"\xa0" + struct.pack("<H", len(body))
    return prefix + bytes([sum(prefix) & 255]) + body


def decode(raw):
    if len(raw) < 8 or raw[0] != 0xA0:
        return None
    n = int.from_bytes(raw[1:3], "little") + 4
    if n < 8 or n > len(raw) or sum(raw[:3]) & 255 != raw[3]:
        return None
    raw = raw[:n]
    if int.from_bytes(raw[5:7], "little") + 7 != n:
        return None
    if sum(raw[4:]) & 255 != 0xAA:
        return None
    return {"command": raw[4], "payload": raw[7:-1], "frame": raw}


def decode_client_hello(raw):
    if len(raw) < 4 or raw[0] != 0xB0 or sum(raw[:3]) & 255 != raw[3]:
        return None
    n = int.from_bytes(raw[1:3], "little")
    if n < 48 or n > len(raw) - 4:
        return None
    tls = raw[4 : 4 + n]
    if tls[0] != 22 or tls[5] != 1:
        return None
    if int.from_bytes(tls[3:5], "big") + 5 != n:
        return None
    if int.from_bytes(tls[6:9], "big") + 9 != n:
        return None
    sid_len = tls[43]
    pos = 44 + sid_len
    if sid_len > 32 or pos + 2 > n:
        return None
    cipher_len = int.from_bytes(tls[pos : pos + 2], "big")
    if cipher_len < 2 or cipher_len % 2 or pos + 2 + cipher_len >= n:
        return None
    ciphers = tls[pos + 2 : pos + 2 + cipher_len]
    return {
        "tls_record_type": tls[0],
        "tls_version": tls[1:3].hex(),
        "tls_record_length": n,
        "client_hello": True,
        "cipher_suites": [ciphers[i : i + 2].hex() for i in range(0, len(ciphers), 2)],
    }
