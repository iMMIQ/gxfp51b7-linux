#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Translation/overlap analysis for tiny images; never authenticates.
Scores are experimental and require independent evaluation before login use.
"""
import argparse
import json
from pathlib import Path
import numpy as np
from scipy.signal import correlate
from scipy.ndimage import rotate, affine_transform
from image_decode import decode
from matcher_eval import preprocess


def prepare(path, background):
    return np.array(preprocess(decode(Path(path).read_bytes()), background),
                    dtype=np.float64).reshape(64, 80)[4:-4, 4:-4]


def match(a, b, max_dx=30, max_dy=24, mask=None):
    if a.shape != (56, 72) or b.shape != a.shape:
        raise ValueError('Unexpected matching image shape')
    if not np.isfinite(a).all() or not np.isfinite(b).all():
        raise ValueError('Nonfinite image')
    h, w = a.shape
    if not 0 <= max_dx < w or not 0 <= max_dy < h:
        raise ValueError('Invalid displacement range')
    dx, dy = np.meshgrid(np.arange(-max_dx, max_dx + 1),
                         np.arange(-max_dy, max_dy + 1))
    y0, y1 = np.maximum(0, dy), np.minimum(h, h + dy)
    x0, x1 = np.maximum(0, dx), np.minimum(w, w + dx)
    by0, by1 = np.maximum(0, -dy), np.minimum(h, h - dy)
    bx0, bx1 = np.maximum(0, -dx), np.minimum(w, w - dx)
    def sums(image, ya, yb, xa, xb):
        integral = np.pad(image.cumsum(0).cumsum(1), ((1, 0), (1, 0)))
        return integral[yb, xb] - integral[ya, xb] - integral[yb, xa] + integral[ya, xa]
    if mask is None:
        area = (y1 - y0) * (x1 - x0)
    else:
        if mask.shape != a.shape or not np.isfinite(mask).all():
            raise ValueError('Invalid rotation mask')
        mask = (mask > .999).astype(np.float64)
        a = a * mask
        area = sums(mask, y0, y1, x0, x1)
    safe_area = np.maximum(area, 1)
    sa = sums(a, y0, y1, x0, x1)
    if mask is None:
        sb = sums(b, by0, by1, bx0, bx1)
        sb2 = sums(b*b, by0, by1, bx0, bx1)
    else:
        sb = correlate(mask, b, mode='full', method='fft')[h-1+dy, w-1+dx]
        sb2 = correlate(mask, b*b, mode='full', method='fft')[h-1+dy, w-1+dx]
    va = np.maximum(0, sums(a*a, y0, y1, x0, x1) - sa*sa/safe_area)
    vb = np.maximum(0, sb2 - sb*sb/safe_area)
    numerator = correlate(a, b, mode='full', method='fft')[h-1+dy, w-1+dx] - sa*sb/safe_area
    denominator = np.sqrt(va*vb)
    scores = np.full(area.shape, -1.0)
    valid = (area >= 1600) & (denominator > 1e-9)
    np.divide(numerator, denominator, out=scores, where=valid)
    index = np.unravel_index(scores.argmax(), scores.shape)
    if not valid[index]:
        return {'correlation': -1.0, 'dx': 0, 'dy': 0, 'overlap': 0}
    return {'correlation': float(np.clip(scores[index], -1, 1)),
            'dx': int(dx[index]), 'dy': int(dy[index]), 'overlap': int(area[index])}


def match_rotated(a, b):
    best = {**match(a, b), 'angle': 0}
    ones = np.ones(a.shape, dtype=np.float64)
    for angle in range(-24, 25, 3):
        if angle == 0:
            continue
        rotated = rotate(a, angle, reshape=False, order=1, mode='constant', cval=0)
        mask = rotate(ones, angle, reshape=False, order=1, mode='constant', cval=0)
        candidate = match(rotated, b, mask=mask)
        if candidate['correlation'] > best['correlation']:
            best = {**candidate, 'angle': angle}
    return best


def affine_image(a, angle, sx, sy, shear):
    """Small bounded deformation, with the same angle convention as rotate.

    The mask excludes interpolation outside the actual sensor area. Parameters
    describe the output-to-input sampling matrix, not a biometric tolerance.
    """
    if a.shape != (56, 72) or not np.isfinite(a).all():
        raise ValueError('Invalid matching image')
    if not (-24 <= angle <= 24 and .94 <= sx <= 1.06 and
            .94 <= sy <= 1.06 and -.03 <= shear <= .03):
        raise ValueError('Deformation outside research bounds')
    radians = np.deg2rad(angle)
    rotation = np.array([[np.cos(radians), np.sin(radians)],
                         [-np.sin(radians), np.cos(radians)]])
    matrix = rotation @ np.array([[sy, shear], [0, sx]])
    center = (np.array(a.shape) - 1) / 2
    offset = center - matrix @ center
    options = dict(matrix=matrix, offset=offset, order=1,
                   mode='constant', cval=0)
    return (affine_transform(a, **options),
            affine_transform(np.ones(a.shape), **options))


def match_affine_set(references, probe):
    """Frozen coarse-to-fine research search; requires fresh validation.

    Only the three strongest rigid matches are refined. Search bounds are
    fixed, so scores cannot select unlimited deformation or tiny overlaps.
    """
    if len(references) == 0:
        raise ValueError('No reference images')
    candidates = [{**match_rotated(a, probe), 'reference_index': i,
                   'sx': 1., 'sy': 1., 'shear': 0.}
                  for i, a in enumerate(references)]
    best = max(candidates, key=lambda item: item['correlation'])
    for coarse in sorted(candidates, key=lambda item: item['correlation'],
                         reverse=True)[:3]:
        i = coarse['reference_index']
        for angle in range(max(-24, coarse['angle'] - 2),
                           min(24, coarse['angle'] + 2) + 1):
            for sx in (.94, .97, 1., 1.03, 1.06):
                for sy in (.94, .97, 1., 1.03, 1.06):
                    for shear in (-.03, 0., .03):
                        transformed, mask = affine_image(references[i], angle, sx, sy, shear)
                        candidate = match(transformed, probe, mask=mask)
                        if candidate['correlation'] > best['correlation']:
                            best = {**candidate, 'reference_index': i,
                                    'angle': angle, 'sx': sx, 'sy': sy,
                                    'shear': shear}
    return best


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--background', required=True)
    p.add_argument('--rotation', action='store_true')
    p.add_argument('captures', nargs='+')
    args = p.parse_args()
    background = decode(Path(args.background).read_bytes())
    images = [prepare(path, background) for path in args.captures]
    pairs = []
    for i in range(len(images)):
        for j in range(i + 1, len(images)):
            pairs.append({'a': Path(args.captures[i]).name,
                          'b': Path(args.captures[j]).name,
                          **(match_rotated if args.rotation else match)(images[i], images[j])})
    print(json.dumps({'authentication_enabled': False, 'pairs': pairs}, indent=2))
