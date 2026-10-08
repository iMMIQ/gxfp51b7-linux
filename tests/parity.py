#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Independent, synthetic decoder and presence-quality regression checks."""

import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

import numpy as np

sys.path.insert(0, str(Path(__file__).parent / "reference"))
from image_decode import decode

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get("GXFP_ORACLE", ROOT / "target/release/examples/oracle"))


def evaluate(operation, **values):
    result = subprocess.run(
        [str(BINARY)],
        input=json.dumps(
            {
                "operation": operation,
                **{k: v.tolist() if isinstance(v, np.ndarray) else v for k, v in values.items()},
            }
        ),
        text=True,
        capture_output=True,
        timeout=5,
    )
    if result.returncode:
        raise ValueError(result.stderr.strip())
    return json.loads(result.stdout)


class RustParityTests(unittest.TestCase):
    def test_nibble_decoder_and_padding(self):
        rng = np.random.default_rng(51)
        raw = bytearray(10560)
        for x in range(80):
            raw[x * 132 : x * 132 + 96] = rng.integers(0, 256, 96, dtype=np.uint8).tobytes()
        self.assertEqual(evaluate("decode", source=list(raw)), decode(raw))
        packed = b"".join(raw[x * 132 : x * 132 + 96] for x in range(80))
        self.assertEqual(evaluate("decode", source=list(packed)), decode(raw))
        raw[96] = 1
        with self.assertRaises(ValueError):
            evaluate("decode", source=list(raw))
        for n in (0, 7679, 10559):
            with self.assertRaises(ValueError):
                evaluate("decode", source=[0] * n)

    def test_quality_uses_upper_medians_on_even_widths(self):
        rng = np.random.default_rng(51)
        background = rng.integers(2500, 4096, 5120, dtype=np.uint16)
        pixels = rng.integers(400, 2400, 5120, dtype=np.uint16)
        delta = (background.astype(float) - pixels).reshape(64, 80)
        expected = delta - np.partition(delta, 40, axis=1)[:, 40:41]
        expected -= np.partition(expected, 32, axis=0)[32:33, :]
        np.testing.assert_allclose(
            evaluate("quality", pixels=pixels, background=background),
            [delta.mean(), np.std(expected)],
            atol=1e-10,
            rtol=0,
        )
        self.assertEqual(
            evaluate("quality", pixels=[3000] * 5120, background=[3000] * 5120), [0, 0]
        )
        with self.assertRaises(ValueError):
            evaluate("quality", pixels=[0], background=[0] * 5120)


if __name__ == "__main__":
    unittest.main(verbosity=2)
