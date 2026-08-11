# Safety Model

This document defines the boundary CLEAN promises to enforce. A feature that violates an invariant here is a safety-model change, not a routine enhancement.

## Protected assets

- Files outside the two fixed user-cache roots.
- Files that were not part of the reviewed scan manifest.
- Files that changed after scanning.
- Recently modified files unless the person explicitly overrides the age policy.
- Terminal state, including raw mode, cursor visibility, and alternate-screen state.
- Private path and application information, which must not leave the machine through CLEAN.

## Trusted inputs

- The current effective user identity returned by the operating system.
- File metadata returned for the exact descriptor-relative entry being evaluated.
- Selection and exact confirmation typed during the current process.

Environment-provided home paths, arbitrary command-line paths, symlink targets, stale metadata, application naming conventions, and cache-directory semantics are not trusted.

## Invariants

1. CLEAN refuses to run cleanup as root.
2. Production roots are constructed internally as `<account-home>/Library/Caches` and `<account-home>/.cache`.
3. Scan traversal does not follow symlinks or cross filesystem device boundaries.
4. Only regular files with one hard link can become candidates.
5. Files newer than the active cutoff do not become candidates.
6. The interface selects no group automatically.
7. The deletion count must be confirmed with an exact generated phrase.
8. Every file is revalidated against device, inode, link count, length, allocation, modification time, type, root, and ancestor shape.
9. Parent traversal for deletion uses directory descriptors opened with `O_NOFOLLOW`.
10. Removal uses `unlinkat` for one final file-name component and never recursively removes a directory.
11. A validation mismatch preserves the entry and appears in the receipt.
12. Cancellation takes effect between individual entries and produces a partial receipt.
13. Normal errors, cancellation, and panics restore the terminal.
14. CLEAN performs no network request or telemetry operation.

## Threats and mitigations

| Threat | Mitigation | Residual limitation |
|---|---|---|
| Crafted `HOME` environment | Resolve account home through the operating system | A compromised operating system is out of scope |
| Broad or relative target | No production arbitrary-path interface; exact-root validation | Adding custom paths would require a new model |
| Stable symlink escape | `symlink_metadata`, no-follow traversal, descriptor-relative removal | A malicious process can still create denial-of-service races |
| Ancestor replaced after scan | Revalidate every ancestor and open it with `O_NOFOLLOW` | POSIX does not provide compare-and-unlink by inode as one operation |
| Final entry replaced | Check identity immediately before descriptor-relative unlink | A hostile same-user process may race the final check; it cannot redirect unlink through a symlink |
| Hard-link accounting or aliasing | Skip any file whose link count is not one | Less space may be offered for cleanup |
| Application writes during cleanup | Skip changed identities; report busy and missing entries | Open files may not yield immediate free-space changes |
| Partial permission | Continue other work and report inaccessible paths | Full Disk Access is not requested automatically |
| Accidental bulk selection | No preselection and explicit review | Select-all remains an intentional keyboard command |
| Misread confirmation | Phrase includes exact file count and requires exact matching | A person can still deliberately confirm the wrong selection |
| Terminal crash | RAII guard plus panic restoration hook | Process termination that bypasses cleanup may require `reset` |

## Non-goals

- Determining whether every third-party cache file is semantically disposable.
- Cleaning system-owned or another user's data.
- Circumventing macOS privacy or permission controls.
- Guaranteeing an exact physical-space increase on compressed, shared, snapshotted, or concurrently active filesystems.
- Defending against a malicious process with the same user privileges and precise race timing as an absolute security boundary.

## Verification expectations

Every safety change needs a temporary fixture proving both the intended deletion and preservation of a sentinel outside the boundary. The minimum suite covers age cutoffs, dry-run immutability, symlink escape, hard links, changed identities, ancestor replacement, cancellation, empty selection, and exact confirmation.

Real-home verification remains scan-only unless the person at the keyboard separately chooses and confirms cleanup.
