#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Install reviewed source and a prepared private guest bundle; PAM remains disabled."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pwd
import shutil
import stat
import subprocess

REPO = Path(__file__).resolve().parents[1]
ROOT = Path('/usr/local/lib/gxfp51b7')
STATE = Path('/var/lib/gxfp51b7')
SHARE = Path('/usr/local/share/gxfp51b7-guest')
DISK = Path('/var/lib/gxfp51b7-vm')
ASSETS = ('le.signed.sgxs', 'le.production.sigstruct',
          'WBDI_Enclave.signed.sgxs', 'WBDI_Enclave.signed.sigstruct',
          'white_list_cert.bin', 'chicago-default-config.bin')


def copy(source, target, mode=0o644, uid=0, gid=0):
    if source.is_symlink() or not source.is_file() or target.is_symlink():
        raise ValueError('Expected regular file: ' + str(source))
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    target.chmod(mode)
    os.chown(target, uid, gid)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--user', required=True)
    args = parser.parse_args()
    if os.geteuid() != 0:
        parser.error('Requires root')
    # Refuse upgrading an existing deployment. In particular this cannot alter
    # the original research deployment simply by being run accidentally.
    for path in (ROOT, STATE, SHARE, DISK, Path('/etc/systemd/system/gxfp51b7-vm.service'),
                 Path('/usr/lib/security/pam_gxfp51b7.so'), Path('/etc/pam.d/gxfp51b7-test')):
        if path.exists() or path.is_symlink():
            parser.error('Existing deployment; explicit migration required: ' + str(path))
    sys_vendor = Path('/sys/class/dmi/id/sys_vendor').read_text().strip()
    product = Path('/sys/class/dmi/id/product_name').read_text().strip()
    if (sys_vendor, product) != ('HUAWEI', 'MACHC-WAX9'):
        parser.error('Only HUAWEI MACHC-WAX9 is supported')
    if not Path('/sys/bus/acpi/devices/GXFP51B7:00').exists():
        parser.error('GXFP51B7 device absent')
    import sys
    sys.path.insert(0, str(REPO / 'src'))
    from admin import valid_user
    user = pwd.getpwnam(valid_user(args.user))
    if user.pw_uid == 0:
        parser.error('Root authentication is not supported')
    import numpy, scipy  # noqa: F401 -- require the system interpreter dependencies
    bundle = args.bundle.resolve()
    manifest = json.loads((bundle / 'assets.json').read_text())
    files = (*ASSETS, 'id_ed25519', 'known_hosts', 'guest.qcow2')
    for name in files:
        path = bundle / name
        if path.is_symlink() or not path.is_file():
            parser.error('Missing regular bundle file: ' + name)
        if name in ASSETS and hashlib.sha256(path.read_bytes()).hexdigest() != manifest.get(name):
            parser.error('Asset digest mismatch: ' + name)
    disk = json.loads(subprocess.check_output(
        ['/usr/bin/qemu-img', 'info', '--output=json', str(bundle / 'guest.qcow2')]))
    if disk.get('format') != 'qcow2' or disk.get('backing-filename'):
        parser.error('Guest must be a stopped, self-contained qcow2 image')
    if '[127.0.0.1]:2228 ' not in (bundle / 'known_hosts').read_text():
        parser.error('Guest host key must be pinned for [127.0.0.1]:2228')
    for name in ('pam_gxfp51b7.so', 'pam_probe', 'pam_sddm_probe'):
        if not (REPO / 'build' / name).is_file():
            parser.error('Run make before installation')
    # A packaged guest is explicitly privileged inside its VM. Its source and
    # authorized key must be reviewed before it is copied into the service.
    try:
        service = pwd.getpwnam('gxfpvm')
        if service.pw_uid == 0:
            raise ValueError('Unsafe VM account')
    except KeyError:
        subprocess.run(['/usr/bin/useradd', '--system', '--user-group',
                        '--home-dir', str(DISK), '--shell', '/usr/bin/nologin',
                        '--groups', 'kvm,sgx', 'gxfpvm'], check=True)
        service = pwd.getpwnam('gxfpvm')
    ROOT.mkdir(mode=0o755)
    STATE.mkdir(mode=0o700)
    DISK.mkdir(mode=0o750)
    os.chown(DISK, service.pw_uid, service.pw_gid)
    private = ROOT / 'guest-private'
    private.mkdir(mode=0o700)
    for source in sorted((REPO / 'src').glob('*.py')):
        copy(source, ROOT / source.name)
    for name in ('id_ed25519', 'known_hosts', 'chicago-default-config.bin'):
        copy(bundle / name, private / name, 0o600)
    for name in ASSETS:
        if name == 'chicago-default-config.bin':
            continue
        directory = 'windows-psw218' if name == 'white_list_cert.bin' else 'sgxs'
        copy(bundle / name, SHARE / directory / name)
    copy(bundle / 'guest.qcow2', DISK / 'guest.qcow2', 0o600, service.pw_uid, service.pw_gid)
    copy(REPO / 'build/pam_gxfp51b7.so', Path('/usr/lib/security/pam_gxfp51b7.so'))
    for name in ('pam_probe', 'pam_sddm_probe'):
        copy(REPO / 'build' / name, ROOT / name, 0o755)
    copy(REPO / 'data/gxfp51b7-vm.service', Path('/etc/systemd/system/gxfp51b7-vm.service'))
    config = {'enabled': False, 'user': user.pw_name, 'uid': user.pw_uid,
              'policy': 'experimental-affine-ncc-v3', 'threshold': .86,
              'reference_count': 15, 'experimental': True}
    from admin import atomic_write
    atomic_write(STATE / 'config.json', json.dumps(config, indent=2) + '\n')
    test = ('auth required pam_shells.so\nauth requisite pam_nologin.so\n'
            'auth requisite pam_faillock.so preauth\n'
            f'auth required pam_gxfp51b7.so user={user.pw_name}\n'
            'account include system-login\n')
    atomic_write(Path('/etc/pam.d/gxfp51b7-test'), test, 0o644)
    subprocess.run(['/usr/bin/systemctl', 'daemon-reload'], check=True)
    print('Runtime installed. SDDM unchanged. Load host helpers and start the VM, then enroll and check.')


if __name__ == '__main__':
    main()
