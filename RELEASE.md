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
3. Confirm version in workspace/package manifests.
4. Run locally:
   - `cargo fmt --all -- --check`
   - `just validate`
   - `just validate-packaging-strict`
   - `cargo build --release -p voxy-app` (with GTK dependencies installed)
5. Ensure CI is green on `main`.
6. Create and push a signed tag: `vX.Y.Z`.
7. Confirm `Release` workflow completed for the tag and uploaded:
   - `voxy-app-<version>-linux-x86_64.tar.gz`
   - `SHA256SUMS.txt`
8. Verify checksum locally:
   - `sha256sum -c SHA256SUMS.txt`
9. Draft GitHub release notes from changelog entries.
10. Record rollback target (last known good tag) in release notes.
11. Complete and attach `docs/RELEASE_EVIDENCE_TEMPLATE.md`.
