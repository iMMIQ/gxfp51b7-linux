# SPDX-License-Identifier: LGPL-3.0-or-later
#!/usr/bin/python
"""Root service readiness check, pinned guest identity, no image/key output."""
from pathlib import Path
import subprocess
import time

runtime = Path('/usr/local/lib/gxfp51b7/guest-private')
args = ['/usr/bin/ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=1',
        '-o', 'StrictHostKeyChecking=yes', '-o', 'IdentitiesOnly=yes',
        '-o', 'IdentityAgent=none', '-o', 'ForwardAgent=no',
        '-o', 'UserKnownHostsFile=' + str(runtime / 'known_hosts'),
        '-i', str(runtime / 'id_ed25519'), '-p', '2228', 'ubuntu@127.0.0.1',
        'test -e /dev/isgx && test -x /home/ubuntu/legacy_load && '
        'test -f /mnt/research/sgxs/WBDI_Enclave.signed.sgxs && '
        'systemctl is-active --quiet gxfp51b7-guest && echo GXFP_READY']
deadline = time.monotonic() + 45
while time.monotonic() < deadline:
    try:
        result = subprocess.run(args, capture_output=True, timeout=4,
                                env={'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8'})
    except subprocess.TimeoutExpired:
        continue
    if result.returncode == 0 and result.stdout.strip() == b'GXFP_READY':
        print('GXFP51B7 isolated runtime ready')
        raise SystemExit(0)
    time.sleep(1)
raise SystemExit('GXFP51B7 isolated runtime readiness timed out')
