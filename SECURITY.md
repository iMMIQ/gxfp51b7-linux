# Security scope

This implementation authenticates a local account by comparing a
captured fingerprint with an enrolled template. Its evidence comes from a small,
same-participant validation set. Population false-accept rate, liveness detection
and spoof resistance require further evaluation.

The supported configuration is one local user account on the validated machine,
with a reviewed guest, original signed enclaves, a root-owned runtime/template
and the tested SDDM PAM layout. The trusted computing base includes host root,
the administrator-provided guest and the ChicagoHS matcher. Host root controls code,
templates and PAM; the guest influences returned image data. SGX provides access
to the original communication key, while the host matcher makes the authentication
decision. A forged fingerprint satisfying that matcher can be accepted.

Normal authentication processes raw image data in memory. Calibration is stored under `/var/lib/gxfp51b7`; fprintd stores the
enrolled print under `/var/lib/fprint`, with root-private access. The
communication key stays inside the vendor enclave. Keep private keys, BIOS
containers, captures and templates in private local storage. Review logs and
share sanitized metadata when reporting problems.

For suspected authentication bypass or sensitive findings, use GitHub's private
vulnerability reporting when enabled, or the repository owner's published
private contact. If a contact is needed, open an issue requesting one and reserve
the technical findings for that private channel. Maintainers should enable
private reporting before public release.

Authentication integration uses a marked SDDM block and retains the password
path. Keep a working password and review the recovery procedure before enabling
fingerprint login. Tests and CI operate on source and offline fixtures;
installation and enablement are explicit administrator steps.
