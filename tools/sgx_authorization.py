#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Offline validation of the original Intel/Goodix launch authorization chain.

No hardware access. Intel root is the public whitelist verification key from:
https://github.com/intel/linux-sgx/blob/sgx_2.18/psw/ae/data/constants/linux/wl_pub.hh
This verifies signatures and matching product IDs, not successful hardware
launch, launch-control MSR compatibility, or key unsealing.
"""

import argparse
import json
import struct
from pathlib import Path
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import ec, utils, rsa, padding
import hashlib
from sgx_pe import reconstruct

ROOT_X_LE = bytes.fromhex("29391e9bcb86d6eb3c1791c88fc95f8cee0c1c75609c16c2186d6731455c36a9")
ROOT_Y_LE = bytes.fromhex("5f09830de122dae4ed9754e6fee2cc935e059984c94f44247a28cf81ca117eb6")


def public_key(x, y, endian):
    return ec.EllipticCurvePublicNumbers(
        int.from_bytes(x, endian), int.from_bytes(y, endian), ec.SECP256R1()
    ).public_key()


def verify(public, signature, data):
    der = utils.encode_dss_signature(
        int.from_bytes(signature[:32], "big"), int.from_bytes(signature[32:], "big")
    )
    public.verify(der, data, ec.ECDSA(hashes.SHA256()))


def validate(cert_path, target_path, launch_path, production_signature=None):
    cert = Path(cert_path).read_bytes()
    if len(cert) < 248 or struct.unpack_from(">4H", cert) != (1, 0, 0, 0):
        raise ValueError("Invalid Intel provider certificate")
    version, kind, provider, product, revision, count = struct.unpack_from(">4H2I", cert, 136)
    if (version, kind, provider) != (1, 1, 0) or not 0 < count <= 2048:
        raise ValueError("Invalid whitelist format")
    if len(cert) != 136 + 16 + 32 * count + 64:
        raise ValueError("Whitelist length mismatch")
    verify(public_key(ROOT_X_LE, ROOT_Y_LE, "little"), cert[72:136], cert[:72])
    verify(public_key(cert[8:40], cert[40:72], "big"), cert[-64:], cert[136:-64])
    target, _, target_sig = reconstruct(target_path)
    launch, _, launch_sig = reconstruct(launch_path)
    if production_signature:
        launch_sig = Path(production_signature).read_bytes()
        if len(launch_sig) != 1808:
            raise ValueError("Invalid production SIGSTRUCT size")
        pub = rsa.RSAPublicNumbers(
            int.from_bytes(launch_sig[512:516], "little"),
            int.from_bytes(launch_sig[128:512], "little"),
        ).public_key()
        pub.verify(
            launch_sig[516:900][::-1],
            launch_sig[:128] + launch_sig[900:1028],
            padding.PKCS1v15(),
            hashes.SHA256(),
        )
        if launch_sig[960:992].hex() != launch["mrenclave"]:
            raise ValueError("Production signature does not match reconstructed launch enclave")
        launch["embedded_mrsigner"] = launch["mrsigner"]
        launch["mrsigner"] = hashlib.sha256(launch_sig[128:512]).hexdigest()
        launch["production_signature_valid"] = True
    launch_product = int.from_bytes(launch_sig[1024:1026], "little")
    launch_attributes = struct.unpack_from("<Q", launch_sig, 928)[0]
    if launch_product != product or not launch_attributes & 0x20:
        raise ValueError("Launch enclave product ID/attributes do not match certificate")
    entries = [cert[152 + i * 32 : 184 + i * 32] for i in range(count)]
    signer = bytes.fromhex(target["mrsigner"])
    if entries[0] != bytes(32) and signer not in entries:
        raise ValueError("Target signer absent from whitelist")
    return {
        "provider_signature_valid": True,
        "whitelist_signature_valid": True,
        "target_signer_authorized": True,
        "matching_launch_product_id": product,
        "whitelist_revision": revision,
        "whitelist_entries": count,
        "target_mrsigner": target["mrsigner"],
        "launch_enclave": launch,
        "hardware_launch_tested": False,
        "key_unsealing_tested": False,
    }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("certificate")
    p.add_argument("target_enclave")
    p.add_argument("launch_enclave")
    p.add_argument(
        "--production-signature", help="Original le_prod_css.bin for the same measured LE image"
    )
    a = p.parse_args()
    print(
        json.dumps(
            validate(a.certificate, a.target_enclave, a.launch_enclave, a.production_signature),
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
