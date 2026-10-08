#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Memory-only capture transport for the root-owned login runtime."""
from collections import deque
from pathlib import Path
import os
import select
import struct
import subprocess
import tempfile
import time
from mailbox import Mailbox, RX, SIZE, encode, decode as decode_packet

ROOT = Path('/usr/local/lib/gxfp51b7')


def capture(root=ROOT, port=2228):
    if os.geteuid() != 0:
        raise PermissionError('Capture requires root')
    runtime = root / 'guest-private'
    sealed = Path('/dev/goodix_bios_sealed').read_bytes()
    if len(sealed) != 885 or sealed[:4] != b'\x04\x00\x02\x00':
        raise RuntimeError('Unexpected BIOS sealed data')
    args = ['/usr/bin/ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=3',
            '-o', 'StrictHostKeyChecking=yes', '-o', 'IdentitiesOnly=yes',
            '-o', 'IdentityAgent=none', '-o', 'ForwardAgent=no',
            '-o', 'UserKnownHostsFile=' + str(runtime / 'known_hosts'),
            '-i', str(runtime / 'id_ed25519'), '-p', str(port), 'ubuntu@127.0.0.1',
            'sudo timeout 12 /home/ubuntu/legacy_load '
            '/mnt/research/sgxs/le.signed.sgxs '
            '/mnt/research/sgxs/le.production.sigstruct '
            '/mnt/research/windows-psw218/white_list_cert.bin '
            '/mnt/research/sgxs/WBDI_Enclave.signed.sigstruct '
            '/mnt/research/sgxs/WBDI_Enclave.signed.sgxs']
    deadline = time.monotonic() + 12
    with tempfile.TemporaryFile() as log:
        p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                             stderr=log, bufsize=0,
                             env={'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8'})
        try:
            def read_exact(n):
                data = b''
                while len(data) < n:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0 or not select.select([p.stdout], [], [], remaining)[0]:
                        raise TimeoutError('Capture deadline reached')
                    chunk = os.read(p.stdout.fileno(), n-len(data))
                    if not chunk:
                        raise RuntimeError('Capture enclave exited unexpectedly')
                    data += chunk
                return data
            def respond(data):
                p.stdin.write(data);p.stdin.flush()
            respond(sealed)
            source = None
            finished = False
            sent_capture = False
            handshake = None
            pending = deque()
            previous = None
            candidate = None
            stable_at = 0
            configured = False
            with Mailbox() as device:
                def poll():
                    nonlocal previous, candidate, stable_at, configured
                    raw = device.mem[RX:SIZE]
                    plain = decode_packet(raw)
                    if plain and plain['command'] == 0x90 and plain['payload'] == b'\x01\x01':
                        configured = True
                    if len(raw)<4 or raw[0]!=0xb0 or sum(raw[:3])&255!=raw[3]:
                        return
                    n = int.from_bytes(raw[1:3], 'little')
                    if not 5 <= n <= SIZE-RX-4:
                        return
                    record = raw[4:4+n]
                    if record == previous:
                        return
                    if n > 512:
                        if record != candidate:
                            candidate = record;stable_at = time.monotonic();return
                        if time.monotonic()-stable_at < .03:
                            return
                    pending.append(record);previous = record
                while True:
                    magic, kind, n = struct.unpack('<III', read_exact(12))
                    if magic != 0x43505247:
                        raise RuntimeError('Invalid capture RPC')
                    if kind == 2:
                        device.query('tls-hello');poll();respond(struct.pack('<q', 0))
                    elif kind == 0:
                        if n > 65536:
                            raise RuntimeError('Invalid receive size')
                        if finished and not sent_capture:
                            configured = False
                            cfg = (runtime / 'chicago-default-config.bin').read_bytes()
                            if len(cfg) != 256:
                                raise RuntimeError('Invalid runtime configuration')
                            packet = encode(0x90, cfg)
                            device.mem[:len(packet)] = packet;device.doorbell()
                            until = time.monotonic() + .3
                            while not configured and time.monotonic()<until:
                                poll();time.sleep(.0003)
                            if not configured:
                                raise RuntimeError('Sensor configuration not acknowledged')
                            packet = encode(0x20, b'\x01\x00')
                            device.mem[:len(packet)] = packet;device.doorbell()
                            sent_capture = True
                        while not pending:
                            if time.monotonic() >= deadline:
                                raise TimeoutError('No capture record')
                            poll();time.sleep(.0003)
                        record = pending.popleft()
                        part = record[:n]
                        if len(record)>n:pending.appendleft(record[n:])
                        respond(struct.pack('<q', len(part))+part)
                    elif kind == 1:
                        if not 0 < n <= 4092:
                            raise RuntimeError('Oversized transmit record')
                        record = read_exact(n);poll()
                        prefix = b'\xb0'+struct.pack('<H', n)
                        packet = prefix+bytes([sum(prefix)&255])+record
                        device.mem[:len(packet)] = packet;device.doorbell()
                        if record[0]==22 and n==85:finished = True
                        until = time.monotonic()+.003
                        while time.monotonic()<until:poll();time.sleep(.0001)
                        respond(struct.pack('<q', n))
                    elif kind == 4:
                        handshake = n
                    elif kind == 5:
                        if n != 10560:
                            raise RuntimeError('Invalid image source size')
                        source = read_exact(n)
                    elif kind == 3:
                        if n != 10573 or handshake != 16 or source is None:
                            raise RuntimeError('Enclave capture failed')
                        break
                    else:
                        raise RuntimeError('Unexpected capture RPC kind')
            p.stdin.close();p.wait(timeout=2)
            log.seek(0);messages = log.read(65536)
            if p.returncode or b'data_from_device_ecall crc check ok' not in messages:
                raise RuntimeError('Image integrity validation failed')
            if b'0x7180' in messages or b'crc check failed' in messages:
                raise RuntimeError('Capture cryptographic validation failed')
            return source
        finally:
            if p.poll() is None:
                p.stdin.close();p.terminate()
                try:p.wait(timeout=2)
                except subprocess.TimeoutExpired:p.kill();p.wait()
