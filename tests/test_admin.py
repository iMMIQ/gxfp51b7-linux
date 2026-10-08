# SPDX-License-Identifier: LGPL-3.0-or-later
import unittest
from admin import enable_text, disable_text


class PamConfigurationTests(unittest.TestCase):
    original = ('#%PAM-1.0\n\nauth        include     system-login\n'
                '-auth optional pam_kwallet5.so\naccount include system-login\n'
                'password include system-login\nsession include system-login\n')

    def test_round_trip_preserves_password_account_session_and_later_edits(self):
        enabled = enable_text(self.original, 'testuser')
        self.assertIn('user=testuser\n', enabled)
        self.assertIn('auth [success=ok default=1]', enabled)
        later_edit = '# later administrator edit\n'
        self.assertEqual(disable_text(enabled + later_edit),
                         self.original + later_edit)

    def test_refuses_unknown_auth_layout(self):
        for text in ('auth include common-auth\n',
                     'auth required pam_deny.so\nauth include system-login\n'):
            with self.assertRaises(ValueError): enable_text(text, 'testuser')

    def test_refuses_username_injection_and_root(self):
        for user in ('root', 'test\nauth sufficient pam_permit.so', 'bad name'):
            with self.assertRaises(ValueError): enable_text(self.original, user)

    def test_refuses_duplicate_and_incomplete_blocks(self):
        enabled = enable_text(self.original, 'testuser')
        with self.assertRaises(ValueError): enable_text(enabled, 'testuser')
        with self.assertRaises(ValueError): disable_text(enabled.replace('# END GXFP51B7 fingerprint login\n', ''))

    def test_disable_without_block_is_unchanged(self):
        self.assertEqual(disable_text(self.original), self.original)
