# Release Checklist

For store/distribution publishing (Flathub, AUR, COPR), use:
- [docs/PUBLISHING.md](docs/PUBLISHING.md)

For support/triage/rollback operations, use:
- [docs/OPERATIONS.md](docs/OPERATIONS.md)

## Core source release

1. Assign release owner and 48h post-release on-call owner.
2. Update `CHANGELOG.md` under a new version heading.
3. Confirm version in workspace/package manifests.
4. Run locally:
   - `cargo fmt --all -- --check`
   - `just validate`
   - `cargo build --release -p voxy-app` (with GTK dependencies installed)
5. Ensure CI is green on `main`.
6. Create and push a signed tag: `vX.Y.Z`.
7. Draft GitHub release notes from changelog entries.
8. Record rollback target (last known good tag) in release notes.
