#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Reject private/binary files and accidental workstation details in release files."""
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN = {'.raw', '.pgm', '.png', '.jpg', '.npz', '.npy', '.qcow2', '.img',
             '.iso', '.sgxs', '.sigstruct', '.dll', '.exe', '.cab', '.bin',
             '.pem', '.key', '.so', '.ko', '.o', '.pyc', '.log', '.zip'}


def main():
    # Include new, nonignored files before the initial commit too.
    files = subprocess.check_output(
        ['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=ROOT)
    seen = set()
    problems = []
    for raw in files.split(b'\0'):
        if not raw:
            continue
        name = raw.decode()
        if name in seen:
            continue
        seen.add(name)
        path = ROOT / name
        if (path.is_symlink() or path.suffix.lower() in FORBIDDEN or
                any(part in ('private', 'work', 'assets', 'outputs') for part in path.parts) or
                path.name.startswith('id_ed25519') or path.name == 'known_hosts'):
            problems.append(name + ': private/binary path')
            continue
        if path.stat().st_size > 512 * 1024:
            problems.append(name + ': unexpected large file')
            continue
        try:
            text = path.read_text()
        except UnicodeDecodeError:
            problems.append(name + ': non-text content')
            continue
        markers = ('-----BEGIN ' + 'OPENSSH PRIVATE KEY-----',
                   '-----BEGIN ' + 'RSA PRIVATE KEY-----',
                   '-----BEGIN ' + 'PRIVATE KEY-----')
        home_paths = re.findall('/' + r'home/([^/\s\"\']+)/', text)
        if (any(marker in text for marker in markers) or
                any(user != 'ubuntu' for user in home_paths)):
            problems.append(name + ': secret or workstation-specific marker')
    if problems:
        raise SystemExit('\n'.join(problems))
    print(f'Release inventory checked: {len(seen)} text files, no forbidden material found.')


if __name__ == '__main__':
    main()
