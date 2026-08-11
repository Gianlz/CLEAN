# GitHub Repository Setup

The tracked files provide community standards and automation. These repository settings must be configured once after the first push because Git cannot express them.

## Repository identity

Suggested description:

> A safety-first macOS cache cleaner with a polished native terminal interface.

Suggested topics:

```text
macos rust tui terminal cache cleaner ratatui privacy apple-silicon open-source
```

Upload `assets/hero.svg` or a raster export as the social preview after reviewing its crop in GitHub's settings.

## Default branch and ruleset

Use `main` as the default branch. Create a branch ruleset for `main` with:

- Pull requests required.
- At least one approving review.
- Conversation resolution required.
- Required status checks for the CI and CodeQL jobs.
- Branches required to be up to date before merge.
- Force pushes and branch deletion blocked.
- Linear history preferred.
- Code-owner review enabled.

The committed `.github/CODEOWNERS` assigns the repository and its security-sensitive paths to `@Gianlz`. Keep it current when the maintainer team changes.

## Actions

- Keep the default workflow token read-only; `release.yml` declares its narrow write permissions explicitly.
- Allow actions from GitHub and verified creators, or allow only the pinned actions already present.
- Require actions to be pinned to full commit SHAs.
- Keep fork pull-request workflows in approval-required mode for first-time contributors.

The CI and release runners use GitHub's standard ARM64 macOS labels. The CodeQL job uses build-free Rust analysis.

## Security features

Enable:

- Dependency graph.
- Dependabot alerts and security updates.
- Secret scanning and push protection.
- Private vulnerability reporting.
- Code scanning with the committed advanced CodeQL workflow.

Do not enable a second CodeQL default setup alongside the committed workflow unless the advanced workflow is removed.

## Releases

- Enable immutable releases.
- Keep tag creation limited to maintainers.
- Use signed semantic version tags when possible.
- Publish through `release.yml` rather than uploading locally built binaries.

The workflow creates a draft, attaches the archive and checksum, generates provenance for a public repository, and only then publishes the release.

## Community features

- Enable Issues for the structured bug and feature forms.
- Enable Discussions when the project needs open-ended support or design conversation.
- Point security researchers to private vulnerability reporting, never a public issue form.
- Add future maintainers to the repository, CODEOWNERS, and governance document when their roles are established.

## README badges

The README uses the canonical `Gianlz/CLEAN` CI and CodeQL badge URLs. Confirm both badges are green after the first workflow run.
