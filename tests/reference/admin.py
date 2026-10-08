# SPDX-License-Identifier: LGPL-3.0-or-later
"""Small, reviewable PAM configuration operations; no import-time writes."""
import os
import re
import tempfile
from pathlib import Path

BEGIN = '# BEGIN GXFP51B7 fingerprint login\n'
END = '# END GXFP51B7 fingerprint login\n'
USERNAME = re.compile(r'[a-z_][a-z0-9_-]*\$?\Z', re.ASCII)


def valid_user(user):
    if not USERNAME.fullmatch(user) or user == 'root':
        raise ValueError('Expected a non-root local account name')
    return user


def enable_text(text, user):
    valid_user(user)
    if BEGIN in text or END in text:
        raise ValueError('Fingerprint block already exists or is incomplete')
    # Only the tested Arch/EndeavourOS layout is accepted. Do not guess PAM jumps.
    auth = [line for line in text.splitlines()
            if line.strip() and not line.lstrip().startswith('#')
            and line.split()[0].lstrip('-') == 'auth']
    if not auth or not re.fullmatch(r'auth\s+include\s+system-login', auth[0]):
        raise ValueError('Unsupported SDDM authentication layout')
    block = BEGIN + (
        'auth required pam_shells.so\n'
        'auth requisite pam_nologin.so\n'
        'auth required pam_env.so\n'
        'auth requisite pam_faillock.so preauth\n'
        f'auth [success=ok default=1] pam_gxfp51b7.so user={user}\n'
        'auth sufficient pam_faillock.so authsucc\n'
    ) + END
    lines = text.splitlines(keepends=True)
    for index, line in enumerate(lines):
        if line.rstrip('\n') == auth[0]:
            lines.insert(index, block)
            return ''.join(lines)
    raise ValueError('Cannot locate PAM insertion point')


def disable_text(text):
    if BEGIN not in text and END not in text:
        return text
    if text.count(BEGIN) != 1 or text.count(END) != 1:
        raise ValueError('Ambiguous or incomplete fingerprint block')
    start = text.index(BEGIN)
    stop = text.index(END)
    if stop < start:
        raise ValueError('Invalid fingerprint block order')
    return text[:start] + text[stop + len(END):]


def atomic_write(path, data, mode=0o600):
    path = Path(path)
    fd, temporary = tempfile.mkstemp(prefix='.gxfp-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(data if isinstance(data, bytes) else data.encode())
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(temporary, mode)
        os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)
