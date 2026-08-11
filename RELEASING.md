# Releasing CLEAN

Releases are built from semantic version tags by `.github/workflows/release.yml` on a GitHub-hosted Apple-silicon macOS runner.

## Prepare

1. Choose the next Semantic Versioning number.
2. Update `package.version` in `Cargo.toml`.
3. Run `cargo check` to refresh the package entry in `Cargo.lock`, then review the lockfile diff.
4. Move relevant entries from `Unreleased` to a dated version in `CHANGELOG.md`.
5. Run the complete local verification gate.
6. Merge the release-preparation pull request into `main`.

## Tag

Create the tag from the reviewed `main` commit. A signed annotated tag is preferred.

```sh
git switch main
git pull --ff-only
git tag -s v0.1.0 -m "CLEAN v0.1.0"
git push origin v0.1.0
```

The tag must exactly match the Cargo package version with a leading `v`. The workflow rejects malformed or mismatched tags.

## Automated release

The workflow:

1. Checks out the exact tag without retained credentials.
2. Tests the locked dependency graph.
3. Builds an optimized ARM64 macOS binary.
4. Packages the binary with README, license, and security policy.
5. Generates a SHA-256 checksum.
6. Generates build provenance for public repositories.
7. Creates a draft GitHub release with all assets attached.
8. Publishes the completed release.

## Verify

Download the archive and checksum from the release, then run:

```sh
shasum -a 256 -c clean-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
gh attestation verify clean-v0.1.0-aarch64-apple-darwin.tar.gz \
  --repo "$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
```

Run `./clean --version`, `./clean doctor`, and an interactive scan without confirming deletion.

## Repository release setting

Enable immutable releases after the remote repository is created. The workflow attaches every asset to a draft before publication so it remains compatible with immutability.
