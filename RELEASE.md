# Release Checklist

For store/distribution publishing (Flathub, AUR, COPR), use:
- [docs/PUBLISHING.md](docs/PUBLISHING.md)

## Core source release

1. Update `CHANGELOG.md` under a new version heading.
2. Confirm version in workspace/package manifests.
3. Run locally:
   - `cargo fmt --all -- --check`
   - `cargo check -p voxy-core -p voxy-audio -p voxy-stt`
   - `cargo test -p voxy-core`
   - `cargo check -p voxy-app` (with GTK dependencies installed)
4. Ensure CI is green on `main`.
5. Create and push a signed tag: `vX.Y.Z`.
6. Draft GitHub release notes from changelog entries.
