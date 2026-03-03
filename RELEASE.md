# Release Checklist

For store/distribution publishing (Flathub, AUR, COPR), use:
- [docs/PUBLISHING.md](docs/PUBLISHING.md)

For support/triage/rollback operations, use:
- [docs/OPERATIONS.md](docs/OPERATIONS.md)
- [docs/RELEASE_EVIDENCE_TEMPLATE.md](docs/RELEASE_EVIDENCE_TEMPLATE.md)

## Core source release

1. Assign release owner and 48h post-release on-call owner.
   - Record both in `docs/RELEASE_EVIDENCE_TEMPLATE.md`.
2. Update `CHANGELOG.md` under a new version heading.
3. Run metadata preflight checks:
   - `just release-preflight version=<X.Y.Z[-RCN]>`
4. Confirm version in workspace/package manifests.
5. Run locally:
   - `cargo fmt --all -- --check`
   - `just validate`
   - `just validate-packaging-strict`
   - `cargo build --release -p voxy-app` (with GTK dependencies installed)
6. Ensure CI is green on `main`.
7. Create and push a signed tag: `vX.Y.Z`.
8. Confirm `Release` workflow completed for the tag and uploaded:
   - `voxy-app-<version>-linux-x86_64.tar.gz`
   - `screenshots/idle.png`
   - `screenshots/recording.png`
   - `screenshots/config.png`
   - `SHA256SUMS.txt`
9. Verify checksum locally:
   - `sha256sum -c SHA256SUMS.txt`
10. Generate release evidence scaffold:
   - `just release-evidence version=<X.Y.Z[-RCN]>`
11. Draft GitHub release notes from changelog entries.
12. Record rollback target (last known good tag) in release notes.
13. Complete and attach `docs/RELEASE_EVIDENCE_TEMPLATE.md`.
