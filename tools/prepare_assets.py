#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Verify user-supplied vendor files and export assets outside the repository."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

GF_HASH = 'd98e873e02b00d03d90ccdd78ac69624676a6b9c3662ce66dfca220551b675cd'
WBDI_HASH = '8811f32411a236b28c61a0f0d9c134c42357e0dcddd71c2c324873e9219618b4'
LE_HASH = '77a293796bf44cfd39f3b416a9569ac1b424094b3e59eea41af772a3de7a92f6'
CFG_HASH = '5732193758646a9a6baf81e305ee613db34442170b1db4927dd5698846ed3fa4'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('goodix', 'intel', 'output'):
        parser.add_argument('--' + arg, type=Path, required=True)
    args = parser.parse_args()
    import pefile
    from sgx_pe import reconstruct
    from sgx_authorization import validate
    output = args.output.resolve()
    repository = Path(__file__).resolve().parents[1]
    if output == repository or repository in output.parents:
        parser.error('Vendor assets must be outside the public repository')
    if output.exists():
        parser.error('Output must be a new directory')
    paths = [(args.goodix / 'gfspi.dll', GF_HASH),
             (args.goodix / 'WBDI_Enclave.signed.dll', WBDI_HASH),
             (args.intel / 'le.signed.dll', LE_HASH)]
    for path, expected in paths:
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError('Unsupported vendor version: ' + path.name)
    validate(args.intel / 'white_list_cert.bin', paths[1][0], paths[2][0],
             args.intel / 'le_prod_css.bin')
    config = bytearray(pefile.PE(str(paths[0][0])).get_data(0x93510 + 12 * 256, 256))
    checksum = (-sum(struct.unpack('<127H', config[:254])) - 0xa5a5) & 0xffff
    config[254:] = struct.pack('<H', checksum)
    if hashlib.sha256(config).hexdigest() != CFG_HASH:
        raise ValueError('Unsupported sensor configuration')
    assets = {'chicago-default-config.bin': bytes(config),
              'white_list_cert.bin': (args.intel / 'white_list_cert.bin').read_bytes(),
              'le.production.sigstruct': (args.intel / 'le_prod_css.bin').read_bytes()}
    for name, path in (('le.signed', paths[2][0]), ('WBDI_Enclave.signed', paths[1][0])):
        _, sgxs, signature = reconstruct(path)
        assets[name + '.sgxs'] = sgxs
        if name != 'le.signed':
            assets[name + '.sigstruct'] = signature
    output.mkdir(mode=0o700, parents=True)
    for name, data in assets.items():
        target = output / name
        target.write_bytes(data)
        target.chmod(0o600)
    manifest = {name: hashlib.sha256(data).hexdigest() for name, data in assets.items()}
    (output / 'assets.json').write_text(json.dumps(manifest, indent=2) + '\n')
    (output / 'assets.json').chmod(0o600)
    print('Verified vendor assets exported. Prepare your own guest, SSH identity and known_hosts next.')


if __name__ == '__main__':
    main()
