#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Fail-closed helper for the PAM runtime. Requires root-owned config."""
from pathlib import Path
import os
import pwd
import resource
import stat
import sys
import time
import json

RUNTIME = Path('/usr/local/lib/gxfp51b7')
STATE = Path('/var/lib/gxfp51b7')
if Path(__file__).resolve().parent == RUNTIME:
    sys.path.insert(0, str(RUNTIME))


def trusted(path):
    for component in [path, *path.parents]:
        s = component.lstat()
        if s.st_uid != 0 or s.st_mode & 0o022 or stat.S_ISLNK(s.st_mode):
            raise PermissionError('Unsafe runtime ownership')
    return path


def main():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    if os.geteuid()!=0 or len(sys.argv)!=2:
        return 2
    user = pwd.getpwnam(sys.argv[1])
    config = json.loads(trusted(STATE / 'config.json').read_text())
    if (not config.get('enabled') or user.pw_name != config.get('user') or
            user.pw_uid != config.get('uid') or user.pw_uid == 0):
        return 2
    for name in ('capture.py', 'pixel_match_eval.py', 'image_decode.py',
                 'matcher_eval.py', 'mailbox.py'):
        trusted(RUNTIME / name)
    for name in ('id_ed25519', 'known_hosts', 'chicago-default-config.bin'):
        trusted(RUNTIME / 'guest-private' / name)
    from capture import capture
    from image_decode import decode
    from matcher_eval import preprocess
    from pixel_match_eval import match_affine_set
    from scipy.ndimage import gaussian_filter
    import numpy as np
    import statistics
    template = np.load(trusted(STATE / 'template.npz'), allow_pickle=False)
    background, images = template['background'], template['images']
    if background.shape!=(5120,) or images.shape!=(15,56,72) or not np.isfinite(images).all() or not np.isfinite(background).all():
        return 2
    if config.get('policy') != 'experimental-affine-ncc-v3' or config.get('threshold') != .86:
        return 2
    # Password-locked accounts must not gain an alternative login route.
    shadow = next((line.split(':') for line in Path('/etc/shadow').read_text().splitlines()
                   if line.split(':', 1)[0] == user.pw_name), None)
    if shadow is None or not shadow[1] or shadow[1].startswith(('!', '*')):
        return 2
    deadline = time.monotonic()+10
    while time.monotonic()<deadline:
        source = capture();px = decode(source)
        drop = statistics.mean(float(b)-p for b,p in zip(background,px))
        contrast = statistics.pstdev(preprocess(px, background))
        if drop<900 or contrast<40:
            continue
        d = (background.astype(np.float64)-np.array(px)).reshape(64,80)
        probe = (gaussian_filter(d,.7)-gaussian_filter(d,2))[4:-4,4:-4]
        score = match_affine_set(images,probe)['correlation']
        if score>=.86:
            return 0
        return 1
    return 1


if __name__=='__main__':
    try:sys.exit(main())
    except Exception:sys.exit(2)
