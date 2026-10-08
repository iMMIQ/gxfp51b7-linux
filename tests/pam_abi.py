#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Validate Rust PAM exports and malformed ABI inputs without authentication I/O."""
import ctypes as c
from pathlib import Path
import unittest

LIBRARY=Path(__file__).resolve().parents[1] / 'build/pam_gxfp51b7.so'


class PamAbiTests(unittest.TestCase):
    def test_exported_callbacks_reject_null_handles(self):
        library=c.CDLL(str(LIBRARY))
        for name in ('pam_sm_authenticate','pam_sm_setcred','pam_sm_acct_mgmt','pam_sm_open_session','pam_sm_close_session','pam_sm_chauthtok'):
            callback=getattr(library,name)
            callback.argtypes=[c.c_void_p,c.c_int,c.c_int,c.POINTER(c.c_char_p)]
            callback.restype=c.c_int
            for argc in (-1,0,1):
                self.assertEqual(callback(None,0,argc,None),26)


if __name__=='__main__':unittest.main(verbosity=2)
