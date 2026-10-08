#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Offline 80x64 matcher evaluation. Reports scores, never authenticates.
Build goodix_sift.c from Sigfrodr/libfprint-goodixtls (LGPL-2.1-or-later)
as a shared library and pass its path with --library.
"""
import argparse
import ctypes as c
import json
from pathlib import Path
import statistics
from image_decode import decode, WIDTH, HEIGHT


class Point(c.Structure):
    _fields_ = [('x', c.c_int16), ('y', c.c_int16), ('desc', c.c_uint8 * 128)]


class Features(c.Structure):
    _fields_ = [('n', c.c_uint), ('pts', c.POINTER(Point))]


def preprocess(pixels, background):
    if len(pixels) != WIDTH * HEIGHT or len(background) != len(pixels):
        raise ValueError('Expected complete 80x64 image and background')
    d = [float(b) - float(p) for p, b in zip(pixels, background)]
    # Use the upper median, matching the reference C preprocessing.
    for y in range(HEIGHT):
        start = y * WIDTH
        m = sorted(d[start:start + WIDTH])[WIDTH // 2]
        d[start:start + WIDTH] = [v - m for v in d[start:start + WIDTH]]
    for x in range(WIDTH):
        m = sorted(d[x::WIDTH])[HEIGHT // 2]
        for y in range(HEIGHT):
            d[y * WIDTH + x] -= m
    return d


def evaluate(library, background_path, paths):
    lib = c.CDLL(str(Path(library).resolve()))
    lib.gx_sift_extract.argtypes = [c.POINTER(c.c_double), c.c_int, c.c_int]
    lib.gx_sift_extract.restype = c.POINTER(Features)
    lib.gx_sift_match.argtypes = [c.POINTER(Features), c.POINTER(Features)]
    lib.gx_sift_match.restype = c.c_int
    lib.gx_sift_free.argtypes = [c.POINTER(Features)]
    background = decode(Path(background_path).read_bytes())
    features, entries = [], []
    try:
        for path in paths:
            px = decode(Path(path).read_bytes())
            d = preprocess(px, background)
            array = (c.c_double * len(d))(*d)
            f = lib.gx_sift_extract(array, WIDTH, HEIGHT)
            if not f:
                raise RuntimeError('Feature extraction failed')
            features.append(f)
            entries.append({'capture': Path(path).name,
                            'mean_drop': statistics.mean(b - p for b, p in zip(background, px)),
                            'ridge_contrast': statistics.pstdev(d),
                            'keypoints': f.contents.n})
        scores = [[lib.gx_sift_match(a, b) for b in features] for a in features]
        return {'authentication_enabled': False, 'captures': entries, 'scores': scores,
                'score_note': 'Self-comparison does not validate recognition; independent same-finger and other-finger presses are required.'}
    finally:
        for f in features:
            lib.gx_sift_free(f)


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--library', required=True)
    p.add_argument('--background', required=True)
    p.add_argument('captures', nargs='+')
    args = p.parse_args()
    print(json.dumps(evaluate(args.library, args.background, args.captures), indent=2))
