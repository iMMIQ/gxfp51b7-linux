# SPDX-License-Identifier: LGPL-3.0-or-later
import unittest
from image_decode import decode, pgm

class ImageDecodeTests(unittest.TestCase):
    def test_nibble_order_and_column_major_layout(self):
        source = bytes.fromhex('3a bc 56 12 78 94') + bytes(7674)
        p = decode(source)
        self.assertEqual([p[i * 80] for i in range(4)], [0xabc, 0x123, 0x456, 0x789])
        self.assertEqual(sum(v != 0 for v in p), 4)
    def test_short_image_rejected(self):
        with self.assertRaises(ValueError): decode(bytes(7679))
    def test_ec_column_slots(self):
        source = bytearray(10560)
        source[132:138] = bytes.fromhex('3a bc 56 12 78 94')
        p = decode(source)
        self.assertEqual([p[i * 80 + 1] for i in range(4)], [0xabc, 0x123, 0x456, 0x789])
        self.assertEqual(sum(v != 0 for v in p), 4)
    def test_nonzero_padding_rejected(self):
        source = bytearray(10560)
        source[96] = 1
        with self.assertRaises(ValueError): decode(source)
    def test_ambiguous_length_rejected(self):
        with self.assertRaises(ValueError): decode(bytes(8000))
    def test_full_scale_pgm_endianness(self):
        raw = pgm(bytes([255]) * 7680)
        self.assertTrue(raw.startswith(b'P5\n80 64\n4095\n'))
        self.assertTrue(raw.endswith(b'\x0f\xff' * 5120))

if __name__ == '__main__': unittest.main()
