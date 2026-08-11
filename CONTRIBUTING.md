# Contributing to CLEAN

Thank you for helping make cache cleanup safer and more legible.

## Before opening code

- Search existing issues and pull requests.
- Open a feature request before investing in a large behavioral change.
- Use the security process instead of a public issue for vulnerabilities.
- Keep the proposal inside CLEAN's narrow authority unless the safety model is explicitly being redesigned.

Small fixes, tests, documentation, accessibility improvements, and terminal compatibility changes can go directly to a pull request.

## Development environment

CLEAN's destructive primitive and integration suite target macOS. Apple-silicon macOS is the supported development and CI environment.

```sh
# From your fork's checkout:
cargo test --locked --all-targets
```

The repository's `rust-toolchain.toml` installs the minimum supported compiler, formatter, and linter through rustup.

## Make a focused change

1. Create a branch from an up-to-date `main`.
2. Add a failing regression test before changing safety behavior.
3. Keep destructive tests inside temporary fixture directories.
4. Implement the smallest change that satisfies the test.
5. Update user documentation and `CHANGELOG.md` when behavior changes.
6. Run the full local gate.

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

## Safety-sensitive changes

Changes to `src/rules.rs`, `src/safety.rs`, `src/scanner.rs`, or `src/cleaner.rs` must describe:

- The authority being added or removed.
- The path, symlink, mutation, permission, and cancellation cases considered.
- The fixture that proves data beyond the approved boundary remains untouched.
- Any new irreversible state and how the person at the keyboard confirms it.

Never use a real home directory for destructive testing. A pull request that relies on manually deleting real cache data is not reviewable evidence.

## Dependencies

Prefer the standard library and existing dependencies. A new dependency must have a concrete benefit, compatible license, maintained source, minimal enabled features, and a committed `Cargo.lock` update.

GitHub Actions must be pinned to immutable commit SHAs with a trailing version comment so Dependabot can update both safely.

## Pull requests

Use the pull-request template completely. Screenshots are welcome for interface changes, but redact usernames, paths, application names, and cache contents.

Maintainers may ask to split unrelated changes. Approval means the change meets the current safety and quality bar; it is not a promise of immediate release.

## Commit style

Use a short imperative subject. Conventional Commit prefixes such as `fix:`, `feat:`, `docs:`, `test:`, and `chore:` are encouraged but not required.

By contributing, you agree that your contribution is licensed under the repository's MIT License and that you will follow the Code of Conduct.
