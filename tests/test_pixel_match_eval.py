# SPDX-License-Identifier: LGPL-3.0-or-later
import unittest
import numpy as np
from pixel_match_eval import match, match_rotated, affine_image, match_affine_set
from scipy.ndimage import gaussian_filter, rotate


class PixelMatchTests(unittest.TestCase):
    def test_translation_recovered_without_wrapping(self):
        a = np.random.default_rng(17).normal(size=(56, 72))
        b = np.zeros_like(a)
        b[3:, 5:] = a[:-3, :-5]
        result = match(a, b)
        self.assertAlmostEqual(result['correlation'], 1)
        self.assertEqual((result['dx'], result['dy']), (-5, -3))
    def test_blank_has_no_match(self):
        self.assertEqual(match_rotated(np.zeros((56, 72)), np.zeros((56, 72)))['correlation'], -1)
    def test_nonfinite_rejected(self):
        a = np.zeros((56, 72));a[0, 0] = float('nan')
        with self.assertRaises(ValueError): match(a, a)
    def test_independent_texture_does_not_match(self):
        r = np.random.default_rng(28)
        a, b = r.normal(size=(2, 56, 72))
        self.assertLess(match_rotated(a, b)['correlation'], .25)

    def test_affine_recovers_known_deformation(self):
        a = gaussian_filter(np.random.default_rng(44).normal(size=(56, 72)), .8)
        b, _ = affine_image(a, 1, 1.03, .97, .03)
        result = match_affine_set([a], b)
        self.assertGreater(result['correlation'], .999)
        self.assertGreaterEqual(result['overlap'], 1600)
        self.assertEqual(result['reference_index'], 0)

    def test_affine_angle_matches_rotate_convention(self):
        a = np.random.default_rng(45).normal(size=(56, 72))
        b, _ = affine_image(a, 12, 1, 1, 0)
        np.testing.assert_allclose(b, rotate(a, 12, reshape=False, order=1), atol=1e-12)

    def test_affine_independent_texture_stays_rejected(self):
        rng = np.random.default_rng(46)
        a, b = rng.normal(size=(2, 56, 72))
        self.assertLess(match_affine_set([a], b)['correlation'], .3)

    def test_affine_rejects_unbounded_deformation(self):
        a = np.zeros((56, 72))
        with self.assertRaises(ValueError): affine_image(a, 0, .5, 1, 0)


if __name__ == '__main__': unittest.main()
