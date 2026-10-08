# Security scope

This experimental implementation adds an authentication route. It has no validated liveness detection, no spoof-resistance qualification and no population false-accept-rate estimate. An accepted correlation is a local comparison result, not proof that a live enrolled person is present.

The supported boundary is one local, non-root account on the validated machine with a reviewed guest, original signed enclaves, root-owned runtime/template and the tested SDDM PAM layout. Host root can change code, templates and PAM. A compromised trusted guest can influence returned image data. Hardware SGX is used to access the original communication key; it does not validate the custom host matcher.

Normal authentication does not save raw images. Enrollment saves a processed template under `/var/lib/gxfp51b7` with root-only access. The communication key stays inside the vendor enclave. Private keys and BIOS containers must never be attached to public issues. Logs should be reviewed for personal information before sharing.

For suspected authentication bypass or sensitive findings, use GitHub's private vulnerability reporting **if the repository owner has enabled it**, or a private contact published by the owner. If neither exists, open an issue requesting a private contact without disclosing exploit details or sensitive files. Do not invent a reporting address. Maintainers should enable private reporting before public release.

The project only modifies a marked SDDM authentication block. Keep a working password and review the documented recovery path before enabling it. Tests and CI do not install or enable authentication.
