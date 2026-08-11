# Support

## Start with read-only diagnostics

```sh
clean --version
clean doctor
clean scan --dry-run
```

These commands do not enter the deletion flow.

## Ask for help

Use the bug-report issue form for reproducible failures and the feature-request form for proposed behavior. Before posting:

- Search existing issues.
- Remove usernames, full paths, installed-application details, secrets, and cache contents.
- Include macOS version, hardware architecture, terminal name, CLEAN version, and exact keys or command.
- State whether the problem appears during scan, review, confirmation, cleanup, cancellation, or terminal restoration.

The project does not provide private one-to-one cleanup advice or certify that a particular third-party application's cache is disposable.

## Security reports

Do not report vulnerabilities in a public issue. Follow [SECURITY.md](SECURITY.md).
