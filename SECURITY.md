# Security Policy

Filesystem cleanup is security-sensitive. Please report potential boundary escapes, confirmation bypasses, unsafe races, privilege problems, or misleading receipts privately.

## Supported versions

| Version | Security support |
|---|---|
| Latest release | Supported |
| `main` | Best-effort pre-release support |
| Older releases | Upgrade before reporting unless the issue blocks upgrade |

## Report privately

Use [GitHub's private vulnerability report](https://github.com/Gianlz/CLEAN/security/advisories/new). Include:

- CLEAN version and macOS version.
- The violated safety invariant.
- A minimal reproduction using disposable fixture data.
- Whether data outside an approved cache root was read, changed, or removed.
- Any proof of concept, crash output, or proposed mitigation.

Do not include real usernames, personal cache contents, secrets, or public exploit details.

If private vulnerability reporting is not yet enabled, open a public issue asking a maintainer for a private contact channel. Do not include vulnerability details in that issue.

## Response process

The project aims to acknowledge a complete report within seven days, validate severity and affected versions, prepare a regression test and fix, and coordinate publication with the reporter. Complex reports may require more time; the maintainer should provide a status update rather than leave the reporter without a response.

Public disclosure should wait until a fix or documented mitigation is available. The project will credit reporters who want credit and will respect requests for anonymity.

## Scope note

The documented limitation that third-party applications may misuse cache directories is not by itself a vulnerability. A bypass of CLEAN's allowlist, no-follow traversal, manifest identity validation, typed confirmation, root refusal, or receipt integrity is in scope.
