#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Root-only GXFP51B7 research transport; fixed, nonpersistent query commands.

Does not enroll or authenticate. Does not implement firmware or key writes.
The EC doorbell requires the pre-existing acpi_call module.
"""
import argparse
import fcntl
import json
import mmap
import os
import stat
import re
import struct
import time
import uuid
from pathlib import Path

BASE = 0x40200000
SIZE = 0x8000
RX = 0x1000
GUID = uuid.UUID("cc58b68a-4479-4893-a8bb-961209db59e5")
QUERIES = {"chip": (0x90, b""), "firmware": (0xA8, bytes.fromhex("70 dc")),
           # ChipRegRead in official gfspi.dll 1.1.151.18: flag + reg16 + length16.
           "sensor-id": (0x82, bytes.fromhex("00 00 00 04 00")),
           # GetMcuState in the same binary, 18004dcf8.
           "state": (0xAE, b"\x55"),
           # TLSServerInit, 180027174: requests a new ephemeral handshake.
           "tls-hello": (0xD0, b"\x00\x00")}


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
    tls = raw[4:4+n]
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
    cipher_len = int.from_bytes(tls[pos:pos+2], "big")
    if cipher_len < 2 or cipher_len % 2 or pos + 2 + cipher_len >= n:
        return None
    ciphers = tls[pos+2:pos+2+cipher_len]
    return {"tls_record_type": tls[0], "tls_version": tls[1:3].hex(),
            "tls_record_length": n, "client_hello": True,
            "cipher_suites": [ciphers[i:i+2].hex() for i in range(0, len(ciphers), 2)]}


class Mailbox:
    def __enter__(self):
        if os.geteuid() != 0:
            raise PermissionError("Requires root")
        if Path("/sys/class/dmi/id/sys_vendor").read_text().strip() != "HUAWEI":
            raise RuntimeError("Unexpected machine vendor")
        if not Path("/sys/bus/acpi/devices/GXFP51B7:00").exists():
            raise RuntimeError("GXFP51B7 absent")
        if Path("/sys/devices/platform/GXFP51B7:00/driver").exists():
            raise RuntimeError("Device already bound; unload diagnostic driver first")
        iomem = Path("/proc/iomem").read_text()
        if not re.search(r"40200000-40207fff\s*:\s*GXFP51B7:00", iomem):
            raise RuntimeError("Expected reserved mailbox resource absent")
        if not Path("/proc/acpi/call").exists():
            raise RuntimeError("acpi_call module unavailable")
        lock_fd = os.open("/run/lock/gxfp51b7-research.lock",
                          os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
        lock_stat = os.fstat(lock_fd)
        if lock_stat.st_uid != 0 or lock_stat.st_mode & 0o022 or not stat.S_ISREG(lock_stat.st_mode):
            os.close(lock_fd)
            raise PermissionError('Unsafe sensor lock ownership or permissions')
        self.lock = os.fdopen(lock_fd, "a")
        fcntl.flock(self.lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        self.fd = os.open("/dev/mem", os.O_RDWR | os.O_SYNC)
        self.mem = mmap.mmap(self.fd, SIZE, flags=mmap.MAP_SHARED,
                             prot=mmap.PROT_READ | mmap.PROT_WRITE, offset=BASE)
        return self

    def __exit__(self, *args):
        self.mem.close()
        os.close(self.fd)
        self.lock.close()

    def doorbell(self):
        # acpi_call's buffer syntax is b followed by contiguous hex digits.
        call = f"\\_SB.SPBA._DSM b{GUID.bytes_le.hex()} 0 2 {{}}"
        with open("/proc/acpi/call", "w") as f:
            f.write(call)
        result = Path("/proc/acpi/call").read_text().rstrip("\x00\n")
        if result.startswith("Error"):
            raise OSError(result)
        return result

    def query(self, name, timeout=2):
        command, payload = QUERIES[name]
        packet = encode(command, payload)
        before = self.mem[RX:RX + 0x1000]
        self.mem[:len(packet)] = packet
        if self.mem[:len(packet)] != packet:
            raise OSError("TX readback mismatch")
        dsm_result = self.doorbell()
        deadline = time.monotonic() + timeout
        frames, seen = [], set()
        activity = False
        ack_seen = False
        while time.monotonic() < deadline:
            raw = self.mem[RX:RX + 0x1000]
            if raw != before:
                activity = True
            frame = decode(raw)
            if name == "tls-hello" and activity:
                hello = decode_client_hello(raw)
                if hello and (ack_seen or raw[:hello["tls_record_length"]+4] != before[:hello["tls_record_length"]+4]):
                    return {"query": name, "doorbell": dsm_result, "frames": frames,
                            **hello, "payload": ""}
            if frame and activity and frame["frame"] not in seen:
                fresh = frame["frame"] != before[:len(frame["frame"])]
                if fresh and frame["command"] == 0xB0 and frame["payload"][:1] == bytes([command]):
                    ack_seen = True
                if not (fresh or ack_seen):
                    continue
                seen.add(frame["frame"])
                frames.append({"command": frame["command"],
                               "payload": frame["payload"].hex(),
                               "frame": frame["frame"].hex()})
                if frame["command"] == command and name != "tls-hello":
                    return {"query": name, "doorbell": dsm_result,
                            "frames": frames, "payload": frame["payload"].hex()}
            time.sleep(0.001)
        raise TimeoutError(f"No fresh {name} response; observed frames={frames}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("query", choices=QUERIES, nargs="?", default="firmware")
    args = parser.parse_args()
    with Mailbox() as device:
        result = device.query(args.query)
    payload = bytes.fromhex(result["payload"])
    if args.query == "chip" and len(payload) == 2:
        result["chip_id"] = f"0x{int.from_bytes(payload, 'little'):04x}"
    if args.query == "firmware":
        result["firmware"] = payload.rstrip(b"\0").decode("ascii", "replace")
    if args.query == "sensor-id" and len(payload) == 4:
        # ChipRegRead swaps byte pairs before init_FPSensor reads bytes 1..3.
        swapped = bytes([payload[1], payload[0], payload[3], payload[2]])
        result["sensor_id"] = f"0x{int.from_bytes(swapped[1:4], 'little'):06x}"
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
