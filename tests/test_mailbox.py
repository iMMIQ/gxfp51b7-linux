# SPDX-License-Identifier: LGPL-3.0-or-later
import unittest
from mailbox import decode, decode_client_hello, encode


class ProtocolTests(unittest.TestCase):
    def test_captured_commands(self):
        self.assertEqual(encode(0x90), bytes.fromhex("a00400a490010019"))
        self.assertEqual(encode(0xA8, bytes.fromhex("70dc")),
                         bytes.fromhex("a00600a6a8030070dcb3"))

    def test_actual_chip_response(self):
        raw = bytes.fromhex("a00600a6900300e0e156")
        frame = decode(raw + bytes(32))
        self.assertEqual(frame["command"], 0x90)
        self.assertEqual(int.from_bytes(frame["payload"], "little"), 0xE1E0)

    def test_actual_firmware_response(self):
        raw = bytes.fromhex("a01700b7a8140047465f39454c4942455f45435f31393032340022")
        self.assertEqual(decode(raw)["payload"], b"GF_9ELIBE_EC_19024\0")

    def test_truncated_and_corrupted_frames(self):
        raw = bytes.fromhex("a00600a6900300e0e156")
        for n in range(len(raw)):
            self.assertIsNone(decode(raw[:n]))
        for index in range(len(raw)):
            damaged = bytearray(raw)
            damaged[index] ^= 1
            self.assertIsNone(decode(bytes(damaged)))
        # Check both length fields even when header checksum is repaired.
        damaged = bytearray(raw + b"\0")
        damaged[1] += 1
        damaged[3] += 1
        self.assertIsNone(decode(bytes(damaged)))

    def test_tls_client_hello(self):
        # Same record layout and suites observed from this EC; zero test random.
        body = b"\x03\x03" + bytes(32) + bytes.fromhex("00 0004 00ae00ff 0100")
        handshake = b"\x01" + len(body).to_bytes(3, "big") + body
        tls = b"\x16\x03\x03" + len(handshake).to_bytes(2, "big") + handshake
        prefix = b"\xb0" + len(tls).to_bytes(2, "little")
        packet = prefix + bytes([sum(prefix) & 255]) + tls
        self.assertEqual(decode_client_hello(packet)["cipher_suites"], ["00ae", "00ff"])
        self.assertIsNone(decode_client_hello(packet[:-1]))
        damaged = bytearray(packet)
        damaged[4+43] = 255
        self.assertIsNone(decode_client_hello(damaged))
        damaged = bytearray(packet)
        damaged[4+45] = 255
        self.assertIsNone(decode_client_hello(damaged))


if __name__ == "__main__":
    unittest.main()
