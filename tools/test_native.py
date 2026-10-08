#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Run the pinned Chicago backend's synthetic regression tests."""

from pathlib import Path
import hashlib
import os
import shlex
import subprocess

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / "crates/backends/native"
BUILD = ROOT / "build/native-tests"


def main():
    for line in (NATIVE / "SHA256SUMS").read_text().splitlines():
        digest, name = line.split("  ", 1)
        if hashlib.sha256((NATIVE / name).read_bytes()).hexdigest() != digest:
            raise SystemExit("Pinned upstream source changed: " + name)
    BUILD.mkdir(parents=True, exist_ok=True)
    flags = shlex.split(subprocess.check_output(["pkg-config", "--cflags", "gio-2.0"], text=True))
    libs = shlex.split(subprocess.check_output(["pkg-config", "--libs", "gio-2.0"], text=True))
    cc = shlex.split(os.environ.get("CC", "cc"))
    includes = ["-I" + str(NATIVE / part) for part in ("chicago", "common")]
    objects = []
    for source in sorted((NATIVE / "chicago").glob("*.c")) + [NATIVE / "common/goodix-crc.c"]:
        target = BUILD / (source.stem + ".o")
        subprocess.run(
            cc + ["-O2", "-std=c11"] + flags + includes + ["-c", str(source), "-o", str(target)],
            check=True,
        )
        objects.append(str(target))
    for source in sorted((NATIVE / "tests").glob("*.c")):
        binary = BUILD / source.stem
        subprocess.run(
            cc
            + ["-O2", "-std=c11"]
            + flags
            + includes
            + [str(source)]
            + objects
            + libs
            + ["-lm", "-o", str(binary)],
            check=True,
        )
        # Public deterministic suite uses synthetic data; optional private fixtures
        # are supplied explicitly in a separate research invocation.
        subprocess.run([str(binary)], check=True, env={"PATH": "/usr/bin:/bin", "LANG": "C.UTF-8"})


if __name__ == "__main__":
    main()
