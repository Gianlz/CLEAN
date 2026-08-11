## Summary

<!-- Explain the user-visible outcome and why this change is needed. -->

## Safety impact

<!-- List any filesystem, permission, privacy, persistence, or irreversible-behavior changes. Write "None" when there are none. -->

## Verification

<!-- Include exact commands and any manual terminal checks. -->

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked --all-targets`
- [ ] `cargo build --locked --release`

## Contributor checklist

- [ ] The smallest useful change is isolated from unrelated cleanup.
- [ ] Destructive behavior is covered by temporary-fixture regression tests.
- [ ] No test or review step deletes real user cache data.
- [ ] Documentation and `CHANGELOG.md` are updated when behavior changes.
- [ ] Logs, screenshots, fixtures, and commits contain no secrets or private paths.
- [ ] New dependencies are justified and committed through `Cargo.lock`.
