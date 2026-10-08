#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Synthetic cross-language regression tests; all inputs are generated locally."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

import numpy as np
from scipy.ndimage import gaussian_filter, rotate
sys.path.insert(0, str(Path(__file__).parent / 'reference'))
from image_decode import decode
from matcher_eval import preprocess
from pixel_match_eval import affine_image, match, match_affine_set

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get('GXFP_BINARY', ROOT / 'build/gxfp51b7'))


def evaluate(operation, **values):
    def array(value):
        return value.tolist() if isinstance(value, np.ndarray) else value
    result = subprocess.run([str(BINARY), 'evaluate'], input=json.dumps(
        {'operation': operation, **{k: array(v) for k, v in values.items()}}),
        text=True, capture_output=True, timeout=30)
    if result.returncode:
        raise ValueError(result.stderr.strip())
    return json.loads(result.stdout)


class RustParityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rng = np.random.default_rng(51)
        cls.texture = cls.rng.normal(0, 40, (56, 72))

    def test_nibble_decoder_and_padding(self):
        raw = bytearray(10560)
        for x in range(80):
            raw[x*132:x*132+96] = self.rng.integers(0, 256, 96, dtype=np.uint8).tobytes()
        self.assertEqual(evaluate('decode', source=list(raw)), decode(raw))
        packed = b''.join(raw[x*132:x*132+96] for x in range(80))
        self.assertEqual(evaluate('decode', source=list(packed)), decode(raw))
        raw[96] = 1
        with self.assertRaises(ValueError): evaluate('decode', source=list(raw))
        for n in (0, 7679, 10559):
            with self.assertRaises(ValueError): evaluate('decode', source=[0]*n)

    def test_gaussian_and_quality(self):
        background = self.rng.integers(2500, 4096, 5120, dtype=np.uint16)
        pixels = self.rng.integers(400, 2400, 5120, dtype=np.uint16)
        delta = (background.astype(float)-pixels).reshape(64, 80)
        expected = (gaussian_filter(delta, .7)-gaussian_filter(delta, 2))[4:-4, 4:-4]
        np.testing.assert_allclose(evaluate('prepare', pixels=pixels, background=background), expected, atol=3e-12, rtol=0)
        quality = evaluate('quality', pixels=pixels, background=background)
        np.testing.assert_allclose(quality, [delta.mean(), np.std(preprocess(pixels, background))], atol=1e-10, rtol=0)

    def test_affine_and_masks(self):
        for angle, sx, sy, shear in ((0, 1, 1, 0), (-24, .94, 1.06, -.03), (9, 1.03, .97, .03), (24, 1, 1, 0)):
            expected, mask = affine_image(self.texture, angle, sx, sy, shear)
            result = evaluate('affine', image=self.texture, angle=angle, sx=sx, sy=sy, shear=shear)
            np.testing.assert_allclose(result['image'], expected, atol=2e-11, rtol=0)
            np.testing.assert_array_equal(result['mask'], mask > .999)
            if sx == sy == 1 and shear == 0:
                np.testing.assert_allclose(result['image'], rotate(self.texture, angle, reshape=False, order=1, mode='constant', cval=0), atol=2e-11, rtol=0)
        with self.assertRaises(ValueError): evaluate('affine', image=self.texture, angle=25)

    def test_translation_and_blank(self):
        for probe in (np.roll(self.texture, (3, -5), axis=(0, 1)), self.rng.normal(0, 40, (56, 72)), np.zeros((56, 72))):
            expected = match(self.texture, probe)
            actual = evaluate('translation', image=self.texture, probe=probe)
            self.assertAlmostEqual(actual['correlation'], expected['correlation'], places=11)
            for key in ('dx', 'dy', 'overlap'): self.assertEqual(actual[key], expected[key])
        with self.assertRaises(ValueError): evaluate('translation', image=[[0]], probe=self.texture)

    def test_affine_search_fifteen_references_and_timing(self):
        references = [self.rng.normal(0, 40, (56, 72)) for _ in range(15)]
        references[7] = self.texture
        probe, _ = affine_image(self.texture, 9, 1.03, .97, .03)
        started = time.monotonic()
        expected = match_affine_set(references, probe)
        python_seconds = time.monotonic()-started
        started = time.monotonic()
        actual = evaluate('match', references=[a.tolist() for a in references], probe=probe)
        rust_seconds = time.monotonic()-started
        self.assertAlmostEqual(actual['correlation'], expected['correlation'], places=10)
        self.assertEqual(actual['correlation'] >= .86, expected['correlation'] >= .86)
        for key in ('reference_index', 'angle', 'sx', 'sy', 'shear', 'dx', 'dy', 'overlap'): self.assertEqual(actual[key], expected[key])
        print(f'\nSynthetic 15-reference search: Python {python_seconds:.3f}s, Rust {rust_seconds:.3f}s', flush=True)
        with self.assertRaises(ValueError): evaluate('match', references=[], probe=probe)

    def test_decisions_on_both_sides_of_threshold(self):
        noise = np.random.default_rng(86).normal(0, 40, (56, 72))
        for correlation, accepted in ((.84, False), (.89, True)):
            probe = correlation*self.texture + np.sqrt(1-correlation**2)*noise
            expected = match_affine_set([self.texture], probe)
            actual = evaluate('match', references=[self.texture.tolist()], probe=probe)
            self.assertAlmostEqual(actual['correlation'], expected['correlation'], places=10)
            self.assertEqual(actual['correlation'] >= .86, accepted)

    def test_npz_interoperability_and_validation(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'template.npz'
            source = Path(temporary) / 'capture.raw'
            source.write_bytes(bytes(10560))
            np.savez(path, background=np.zeros(5120, dtype=np.uint16), images=np.zeros((15, 56, 72)))
            result = subprocess.run([str(BINARY), 'score', '--template', str(path), '--source', str(source)], text=True, capture_output=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(json.loads(result.stdout)['accepted'])
            for background, images in ((np.zeros(5119, dtype=np.uint16), np.zeros((15, 56, 72))), (np.zeros(5120, dtype=np.uint16), np.full((15, 56, 72), np.nan)), (np.zeros(5120), np.zeros((15, 56, 72)))):
                np.savez(path, background=background, images=images)
                result = subprocess.run([str(BINARY), 'score', '--template', str(path), '--source', str(source)], capture_output=True, timeout=5)
                self.assertEqual(result.returncode, 2)


if __name__ == '__main__': unittest.main(verbosity=2)
