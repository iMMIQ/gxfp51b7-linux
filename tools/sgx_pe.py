#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Offline PE enclave layout verifier and SGXS exporter. No hardware access.

Reconstructs Intel Windows SGX metadata 2.1 layouts used by the original Goodix
modules. The resulting measurement MUST match the original RSA-signed hash.
Based on the documented Intel ABI and Fortanix PE layout description; this is
an independent Python implementation. Does not initialize SGX or unseal keys.
"""

import argparse
import hashlib
import json
import struct
from pathlib import Path

import pefile
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import padding, rsa

PAGE = 4096


def align(n):
    return (n + PAGE - 1) & -PAGE


def reconstruct(path):
    raw = Path(path).read_bytes()
    pe = pefile.PE(data=raw)
    if pe.FILE_HEADER.Machine != 0x8664 or pe.OPTIONAL_HEADER.Magic != 0x20B:
        raise ValueError("Requires a 64-bit Intel PE enclave")
    metadata = [s for s in pe.sections if s.Name.rstrip(b"\0") == b"sgxmeta"]
    if len(metadata) != 1:
        raise ValueError("Expected one sgxmeta section")
    meta = metadata[0].get_data()
    fields = struct.unpack_from("<13I", meta)
    if fields[:4] != (0x635D0E4C, 0x86A80294, 2, 1) or fields[4] != 1876:
        raise ValueError("Only the verified Intel Windows SGX metadata 2.1 layout is supported")
    threads, policy, nssa, ssa_frame, stack, heap = fields[5:11]
    if not 0 < threads <= 64 or nssa != 2 or ssa_frame != 1:
        raise ValueError("Unsupported thread/SSA layout")
    if stack <= 0 or stack > 0x1000000 or heap > 0x4000000:
        raise ValueError("Unexpected memory requirements")
    sig = meta[68:1876]
    pub = rsa.RSAPublicNumbers(
        int.from_bytes(sig[512:516], "little"), int.from_bytes(sig[128:512], "little")
    ).public_key()
    pub.verify(sig[516:900][::-1], sig[:128] + sig[900:1028], padding.PKCS1v15(), hashes.SHA256())
    expected = sig[960:992].hex()
    sections = [s for s in pe.sections if s is not metadata[0]]
    heap_off = max(s.VirtualAddress + align(s.Misc_VirtualSize) for s in sections)
    tcs_start = heap_off + align(heap) + 0x10000
    tls_dir = pe.DIRECTORY_ENTRY_TLS.struct
    tls_rva = tls_dir.StartAddressOfRawData - pe.OPTIONAL_HEADER.ImageBase
    tls_raw_size = tls_dir.EndAddressOfRawData - tls_dir.StartAddressOfRawData
    if tls_raw_size < 104:
        raise ValueError("TLS template is too small")
    tls_size = align(tls_raw_size + tls_dir.SizeOfZeroFill)
    ssa_rel = PAGE + tls_size + 0x10000
    stack_rel = ssa_rel + nssa * PAGE + 0x10000
    thread_size = stack_rel + align(stack)
    enclave_size = 1 << (tcs_start + threads * thread_size - 1).bit_length()
    tls = bytearray(pe.get_data(tls_rva, tls_raw_size))
    # Windows TRTS template fields are relative to TCS/enclave base.
    struct.pack_into(
        "<5Q", tls, 8, thread_size, thread_size, stack_rel, ssa_rel, ssa_rel + PAGE - 184
    )
    struct.pack_into("<Q", tls, 48, PAGE)
    tls[56] = policy
    struct.pack_into("<QQ", tls, 64, heap_off, enclave_size)
    struct.pack_into("<QII", tls, 88, 0x1030, 1, align(heap))
    reloc_pages = {
        entry.rva // PAGE
        for block in pe.DIRECTORY_ENTRY_BASERELOC
        for entry in block.entries
        if entry.type
    }
    header = bytearray(raw[: pe.OPTIONAL_HEADER.SizeOfHeaders])
    if len(header) > PAGE:
        raise ValueError("Multi-page PE headers have not been verified")
    struct.pack_into("<I", header, pe.OPTIONAL_HEADER.get_field_absolute_offset("CheckSum"), 0)
    struct.pack_into("<II", header, pe.OPTIONAL_HEADER.DATA_DIRECTORY[4].get_file_offset(), 0, 0)
    pages = []

    def add(rva, data, flags, measured=True):
        pages.append((rva, flags, measured, bytes(data).ljust(PAGE, b"\0")[:PAGE]))

    add(0, header, 0x201)
    patched = False
    for section in sections:
        flags = 0x200
        flags |= (section.Characteristics >> 30) & 1
        flags |= ((section.Characteristics >> 31) & 1) << 1
        flags |= ((section.Characteristics >> 29) & 1) << 2
        data = bytearray(section.get_data().ljust(align(section.Misc_VirtualSize), b"\0"))
        if section.VirtualAddress <= tls_rva < section.VirtualAddress + section.Misc_VirtualSize:
            start = tls_rva - section.VirtualAddress
            data[start : start + len(tls)] = tls
            patched = True
        for off in range(0, align(section.Misc_VirtualSize), PAGE):
            rva = section.VirtualAddress + off
            add(rva, data[off : off + PAGE], flags | (2 if rva // PAGE in reloc_pages else 0))
    if not patched:
        raise ValueError("TLS template not inside a mapped PE section")
    for off in range(0, align(heap), PAGE):
        add(heap_off + off, b"", 0x203, False)
    entry = next(
        symbol.address
        for symbol in pe.DIRECTORY_ENTRY_EXPORT.symbols
        if symbol.name == b"enclave_entry"
    )
    tcs_addresses = []
    for i in range(threads):
        tcs_rva = tcs_start + i * thread_size
        tcs_addresses.append(tcs_rva)
        tcs = bytearray(PAGE)
        struct.pack_into(
            "<QQQIIQQQQII",
            tcs,
            0,
            0,
            0,
            tcs_rva + ssa_rel,
            0,
            nssa,
            entry,
            0,
            tcs_rva + PAGE,
            tcs_rva + PAGE,
            4095,
            4095,
        )
        add(tcs_rva, tcs, 0x100)
        for off in range(0, tls_size, PAGE):
            add(tcs_rva + PAGE + off, b"", 0x203)
        for i_ssa in range(nssa):
            add(tcs_rva + ssa_rel + i_ssa * PAGE, b"", 0x203)
        for off in range(0, align(stack), PAGE):
            add(tcs_rva + stack_rel + off, b"\xcc" * PAGE, 0x203)
    sgxs = bytearray(b"ECREATE\0" + struct.pack("<IQ", ssa_frame, enclave_size) + bytes(44))
    for rva, flags, measured, data in pages:
        sgxs += b"EADD\0\0\0\0" + struct.pack("<QQ", rva, flags) + bytes(40)
        if measured:
            for off in range(0, PAGE, 256):
                sgxs += b"EEXTEND\0" + struct.pack("<Q", rva + off) + bytes(48)
                sgxs += data[off : off + 256]
    actual = hashlib.sha256(sgxs).hexdigest()
    if actual != expected:
        raise ValueError(
            f"Signed measurement mismatch: expected {expected}, reconstructed {actual}"
        )
    info = {
        "file": Path(path).name,
        "file_sha256": hashlib.sha256(raw).hexdigest(),
        "rsa_signature_valid": True,
        "measurement_matches_signature": True,
        "mrenclave": actual,
        "mrsigner": hashlib.sha256(sig[128:512]).hexdigest(),
        "metadata_version": "2.1",
        "enclave_size": enclave_size,
        "mapped_pages": len(pages),
        "ssa_frame_size": ssa_frame,
        "threads": threads,
        "tcs_offsets": tcs_addresses,
        "enclave_entry_offset": entry,
        "sigstruct_attributes": struct.unpack_from("<QQ", sig, 928),
        "hardware_loading_tested": False,
        "key_unsealing_tested": False,
    }
    return info, bytes(sgxs), sig


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("enclave")
    parser.add_argument(
        "--out-dir", type=Path, help="Export verified SGXS, SIGSTRUCT and layout JSON"
    )
    args = parser.parse_args()
    info, sgxs, sig = reconstruct(args.enclave)
    if args.out_dir:
        args.out_dir.mkdir(parents=True, exist_ok=True)
        stem = Path(args.enclave).stem
        (args.out_dir / f"{stem}.sgxs").write_bytes(sgxs)
        (args.out_dir / f"{stem}.sigstruct").write_bytes(sig)
        (args.out_dir / f"{stem}.layout.json").write_text(json.dumps(info, indent=2) + "\n")
    print(json.dumps(info, indent=2))


if __name__ == "__main__":
    main()
