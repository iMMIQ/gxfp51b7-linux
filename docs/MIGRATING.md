# Migrating an existing deployment to Rust

Version 0.2 uses a compiled Rust host executable and PAM module. The enclave
assets, guest disk, SSH identity and v3 template format carry over from version
0.1. The read-only kernel helper and guest loader keep their established ABI.

## Prepare and validate

Run `make check`, then place the new executable at
`/usr/local/lib/gxfp51b7/gxfp51b7`, owned by root with mode 0755. Preserve copies
of the current PAM module, service unit, configuration and commissioning report
in the root-private backup directory. Keep the enrolled template in its current
root-private location.

The executable uses `guest-private/` for a standard deployment. During migration
it also recognizes the earlier root-owned `work/research/sgx-vm/` directory.
Its existing identity and pinned host key provide continuity with the same guest.

Run these checks:

```sh
sudo /usr/local/lib/gxfp51b7/gxfp51b7 runtime-ready
sudo /usr/local/lib/gxfp51b7/gxfp51b7 capture-check
sudo /usr/local/lib/gxfp51b7/gxfp51b7 verify YOUR_ACCOUNT
```

`verify` returns 0 for a matching finger, 1 for a completed rejection and 2 for
an unavailable backend or incompatible enrollment. Check an empty sensor, the
enrolled finger and different fingers while maintaining the current template.

Place the new PAM library in a separate root-owned staging directory. Create a
separate PAM test service pointing to that staged library and the explicit
`user=YOUR_ACCOUNT` argument. Build the probe with that service name and check
its authentication/account results. This exercises the new module before it
becomes the module referenced by SDDM.

## Activate

After validation, install `build/pam_gxfp51b7.so` at
`/usr/lib/security/pam_gxfp51b7.so` with root ownership and mode 0644. Install the
Rust service unit from `data/gxfp51b7-vm.service`, reload systemd and restart the
fingerprint VM service. Run `runtime-ready` and the isolated checks again.
The existing marked SDDM branch resolves the updated module through the same
module filename. Password, account and session configuration remain available.

Refresh commissioning evidence with `gxfp51b7 check`. Its report binds the Rust
executable, PAM library, configuration, probe and template to their digests.
The original `.npz` enrollment is compatible with the Rust template reader.

For a failed migration, restore the saved PAM module and service unit, reload
systemd and restart the fingerprint VM. The saved source remains available for
that earlier deployment. Complete recovery and removal steps are in
[the installation guide](INSTALL.md#recovery-and-removal).
