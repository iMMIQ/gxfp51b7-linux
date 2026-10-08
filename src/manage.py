#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Administrator-operated enrollment, live validation and SDDM integration."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pwd
import resource
import subprocess
import sys

ROOT = Path('/usr/local/lib/gxfp51b7')
STATE = Path('/var/lib/gxfp51b7')
if Path(__file__).resolve().parent == ROOT:
    sys.path.insert(0, str(ROOT))
from admin import atomic_write, enable_text, disable_text, valid_user
from verify import trusted


def save(name, value):
    atomic_write(STATE / name, json.dumps(value, indent=2) + '\n')


def config():
    return json.loads(trusted(STATE / 'config.json').read_text())


def fingerprint():
    digest = hashlib.sha256()
    for name in ('config.json', 'template.npz'):
        digest.update(trusted(STATE / name).read_bytes())
    for name in sorted(ROOT.glob('*.py')):
        digest.update(trusted(name).read_bytes())
    digest.update(trusted(Path('/usr/lib/security/pam_gxfp51b7.so')).read_bytes())
    digest.update(trusted(ROOT / 'pam_probe').read_bytes())
    digest.update(trusted(Path('/etc/pam.d/gxfp51b7-test')).read_bytes())
    return digest.hexdigest()


def active():
    subprocess.run(['/usr/bin/systemctl', 'is-active', '--quiet',
                    'gxfp51b7-vm.service'], check=True)


def enroll(user):
    import io
    import statistics
    import numpy as np
    from scipy.ndimage import gaussian_filter
    from capture import capture
    from image_decode import decode
    from matcher_eval import preprocess
    user = pwd.getpwnam(valid_user(user))
    if user.pw_uid == 0:
        raise ValueError('Root enrollment is not supported')
    if '# BEGIN GXFP51B7' in Path('/etc/pam.d/sddm').read_text():
        raise ValueError('Disable fingerprint login before changing enrollment')
    active()
    input('Remove all fingers, then press Enter to capture the background. ')
    background = np.array(decode(capture()), dtype=np.uint16)
    images = []
    for index in range(15):
        input(f'Place the same index finger, then press Enter ({index + 1}/15). ')
        pixels = decode(capture())
        drop = statistics.mean(float(b) - p for b, p in zip(background, pixels))
        contrast = statistics.pstdev(preprocess(pixels, background))
        if drop < 900 or contrast < 40:
            raise ValueError('Insufficient contact; enrollment stopped, old template unchanged')
        delta = (background.astype(np.float64) - np.array(pixels)).reshape(64, 80)
        images.append((gaussian_filter(delta, .7) - gaussian_filter(delta, 2))[4:-4, 4:-4])
        input('Lift the finger completely, wait two seconds, then press Enter. ')
    stream = io.BytesIO()
    np.savez(stream, background=background, images=np.array(images, dtype=np.float64))
    # Disable the old configuration before replacing the template.
    save('config.json', {'enabled': False})
    atomic_write(STATE / 'template.npz', stream.getvalue())
    save('config.json', {'enabled': True, 'user': user.pw_name, 'uid': user.pw_uid,
                        'policy': 'experimental-affine-ncc-v3', 'threshold': .86,
                        'reference_count': 15, 'experimental': True})
    (STATE / 'live-validation.json').unlink(missing_ok=True)
    print('Enrollment stored privately. Run check before enabling SDDM.')


def check():
    active()
    user = valid_user(config()['user'])
    cases = [('empty', False, 'Remove all fingers'),
             ('same', True, 'Place the enrolled index finger'),
             ('middle', False, 'Place a different middle finger'),
             ('ring', False, 'Place a different ring finger'),
             ('little', False, 'Place a different little finger')]
    results = []
    (STATE / 'live-validation.json').unlink(missing_ok=True)
    initial = fingerprint()
    for label, expected, prompt in cases:
        input(prompt + ', then press Enter. ')
        completed = subprocess.run([str(ROOT / 'pam_probe'), user],
                                   capture_output=True, text=True, timeout=22)
        lines = completed.stdout.strip().splitlines()
        if len(lines) != 1 or not lines[0].startswith('PAM_RESULT='):
            raise ValueError('PAM probe did not complete')
        result = int(lines[0].split('=')[1])
        if completed.returncode not in (0, 1) or (result == 0) != (completed.returncode == 0):
            raise ValueError('Inconsistent PAM result')
        accepted = result == 0
        if accepted != expected:
            raise ValueError(f'Unexpected result for {label}; SDDM has not been enabled')
        results.append({'case': label, 'accepted': accepted})
        print('Expected result observed.')
    if fingerprint() != initial:
        raise ValueError('Runtime changed during validation')
    save('live-validation.json', {'fingerprint': initial, 'results': results})
    print('Five live checks passed. This is not a general false-accept-rate study.')


def enable():
    active()
    current = config()
    if not current.get('enabled'):
        raise ValueError('Enrollment is disabled')
    report = json.loads(trusted(STATE / 'live-validation.json').read_text())
    expected = [('empty', False), ('same', True), ('middle', False),
                ('ring', False), ('little', False)]
    if (report['fingerprint'] != fingerprint() or
            [(r['case'], r['accepted']) for r in report['results']] != expected):
        raise ValueError('Fresh live validation is required for this runtime and template')
    path = trusted(Path('/etc/pam.d/sddm'))
    original = path.read_text()
    updated = enable_text(original, current['user'])
    backup = STATE / 'backups'
    backup.mkdir(mode=0o700, exist_ok=True)
    trusted(backup)
    target = backup / 'sddm-before-fingerprint'
    if not target.exists():
        atomic_write(target, original)
    atomic_write(path, updated, 0o644)
    print('SDDM fingerprint branch enabled; password authentication retained.')


def disable():
    path = trusted(Path('/etc/pam.d/sddm'))
    atomic_write(path, disable_text(path.read_text()), 0o644)
    if (STATE / 'config.json').exists():
        current = config()
        current['enabled'] = False
        save('config.json', current)
    subprocess.run(['/usr/bin/systemctl', 'disable', '--now', 'gxfp51b7-vm.service'], check=True)
    print('Fingerprint branch removed and VM stopped. Password login retained.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    commands.add_parser('enroll').add_argument('--user', required=True)
    for command in ('check', 'enable', 'disable'):
        commands.add_parser(command)
    args = parser.parse_args()
    if Path(__file__).resolve().parent != ROOT:
        parser.error('Use the installed root-owned manage.py, not a checkout copy')
    if os.geteuid() != 0:
        parser.error('Requires root')
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    trusted(ROOT)
    trusted(STATE)
    if args.command == 'enroll':
        enroll(args.user)
    else:
        globals()[args.command]()


if __name__ == '__main__':
    try:
        main()
    except (Exception, KeyboardInterrupt) as error:
        raise SystemExit(str(error) or 'Cancelled')
