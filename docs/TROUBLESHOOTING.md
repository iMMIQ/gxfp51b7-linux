# Troubleshooting

| Symptom | Check |
| --- | --- |
| Installer refuses this model | Confirm `MACHC-WAX9`, BIOS 1.24 and ACPI `GXFP51B7`. Other layouts need separate investigation; do not remove guards. |
| `/dev/goodix_bios_sealed` missing | Matching host headers, DKMS result, module signing and `goodix_bios_read` loading. |
| Mailbox unavailable | `acpi_call`, `/proc/acpi/call`, reserved GXFP51B7 range in `/proc/iomem`, and no competing bound diagnostic module. |
| QEMU cannot allocate EPC | BIOS SGX setting, host KVM/virtual EPC support and another SGX VM consuming EPC. Never load guest `isgx` on the host. |
| Guest readiness timeout | Guest kernel/module version, SSH identity permissions, pinned host key, executable loader, 9p share and guest prepare service. |
| Guest public-key login fails | `.ssh`/authorized_keys owner must be `ubuntu`, not root; modes 0700/0600. Pin the actual guest host key. |
| Missing runtime NumPy/SciPy | Install packages for `/usr/bin/python`; the PAM helper ignores development virtual environments. |
| Insufficient contact | Lift fully between presses, place the enrolled finger gently and capture an empty enrollment background. |
| SDDM layout refused | The script accepts the tested Arch `system-login` layout. Review other distributions separately. |
| No fingerprint prompt in SDDM | Submit an empty password first. Some themes block this; the tested eos-breeze theme does not. |
| Fingerprint rejected | Wait for password fallback. Repeated failures require fresh validation/enrollment rather than reducing the fixed threshold. |

Useful metadata commands:

```sh
systemctl status gxfp51b7-vm.service
journalctl -u gxfp51b7-vm.service --no-pager
dkms status
```

Review output locally before sharing. Do not publish captures, templates, private SSH keys, sealed BIOS blobs, guest disks or vendor files. The [installation guide](INSTALL.md#recovery-and-removal) includes recovery steps.
