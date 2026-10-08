# Troubleshooting

| Symptom | Check |
| --- | --- |
| Installer refuses this model | Confirm `MACHC-WAX9`, BIOS 1.24 and ACPI `GXFP51B7`. The checks use the validated model and resource layout. |
| `/dev/goodix_bios_sealed` missing | Matching host headers, DKMS result, module signing and `goodix_bios_read` loading. |
| Mailbox unavailable | `acpi_call`, `/proc/acpi/call`, reserved GXFP51B7 range in `/proc/iomem`, and an unloaded diagnostic module. |
| QEMU EPC allocation failure | BIOS SGX setting, host KVM/virtual EPC support and another SGX VM consuming EPC. Run `isgx` inside the guest, with the current host SGX manager handling physical EPC. |
| Guest readiness timeout | Guest kernel/module version, SSH identity permissions, pinned host key, executable loader, 9p share and guest prepare service. |
| Guest public-key login fails | `.ssh`/authorized_keys owner must be `ubuntu`; modes 0700/0600. Pin the actual guest host key. |
| Build or offline Python dependency missing | Install the Rust toolchain and PAM headers, then use `requirements-tools.txt` for the offline preparation/test environment. |
| Insufficient contact | Lift fully between presses, place the enrolled finger gently and capture an empty enrollment background. |
| SDDM layout refused | The administrator command accepts the tested Arch `system-login` layout. Review other distributions separately. |
| No fingerprint prompt in SDDM | Submit an empty password first. Some themes block this; eos-breeze accepts the empty submission. |
| Fingerprint rejected | Wait for password fallback. Repeated failures require fresh standard PAM validation and a complete 12-stage fprintd enrollment. |

Useful metadata commands:

```sh
systemctl status gxfp51b7-vm.service
journalctl -u gxfp51b7-vm.service --no-pager
dkms status
```

Review output locally before sharing. Keep captures, templates, private SSH keys, sealed BIOS blobs, guest disks and vendor files in private local storage. The [installation guide](INSTALL.md#recovery-and-removal) includes recovery steps.
