# Migrating to the consolidated fprintd implementation

Version 0.3 uses one authentication path: the Rust acquisition worker, ChicagoHS,
libfprint TOD and standard fprintd/PAM. Existing Chicago fprintd enrollments keep
their device identity and private print format. Account configuration reads the
existing username, UID and enabled state. Existing NPZ files supply only the
commissioned empty-sensor background.

## Build and stage

Run `make check`. Preserve the current helper, adapter, SDDM file, configuration,
service drop-ins and probes in a root-private backup directory. Install the new
helper and adapter atomically as described in the [fprintd guide](FPRINTD.md),
and copy the new PAM probes to the root-owned runtime directory.

The new probe returns the Linux-PAM result directly: 0 for success, 7 for an
authentication error, 9 for an unavailable result and 11 for exhausted
attempts. With `max-tries=1`, standard `pam_fprintd` returns 11 after a mismatch.
Commissioning expects that mismatch result and uses these distinctions to reject backend failures during different-finger controls.

Configure `/etc/pam.d/gxfp51b7-test` with the same standard module selected for
SDDM and the normal account include:

```text
auth required /usr/lib/security/pam_fprintd.so max-tries=1 timeout=20
account include system-login
```

On a deployment with the staged official module at
`/usr/local/lib/gxfp51b7/pam_fprintd.so`, use that exact path in the test service.
The administrator commands select that root-owned staged module when present,
and otherwise select the distribution module. Commissioning verifies that the
test service references the selected module.

## Validate and activate

Restart only the fingerprint daemon, confirm the pinned guest with
`gxfp51b7 runtime-ready`, and run `capture-check`. Run `gxfp51b7 check` for the
empty sensor, enrolled finger and three different fingers. Existing configured
SDDM branches can continue using the standard PAM module during these checks;
changing their module requires a saved recovery copy and fresh qualification.

For a new account or renewed calibration, use the fresh-deployment sequence in
[INSTALL.md](INSTALL.md): clear the marked branch, start the VM, calibrate, enroll
through fprintd, check and enable. The original private guest identity and enclave
assets remain part of the commissioned runtime.

The root-owned `work/research/sgx-vm` location and existing capture lock name
remain supported for the established deployment. The previous affine matcher,
custom PAM implementation and research comparison commands are retained in Git
history. Their generated libraries can be removed from the build directory after
migration. Preserve a working password and the saved configuration for rollback.
