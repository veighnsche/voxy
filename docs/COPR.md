# COPR Packaging and Publishing

This document describes how to publish `voxy-app` to Fedora COPR from this repo.

## Prerequisites

- Fedora account with COPR enabled
- `copr-cli` installed and configured (`~/.config/copr`)
- `rpmbuild`, `git`, and Rust toolchain available

```bash
sudo dnf install -y copr-cli rpm-build
copr-cli whoami
```

## 1. Create a COPR project (one-time)

```bash
copr-cli create \
  --chroot fedora-41-x86_64 \
  --chroot fedora-42-x86_64 \
  --repo "voxy-app experimental builds" \
  veighnsche/voxy
```

Adjust owner/project/chroots as needed.

## 2. Preflight release metadata

Before building an SRPM, verify version/changelog consistency:

```bash
just release-preflight version=1.0.0-RC2
```

## 3. Build SRPM from a release tag

The repo includes a maintained spec file at `packaging/rpm/voxy-app.spec`.

```bash
just rpm-srpm ref=v1.0.0-RC2
```

This writes an SRPM under `target/rpm-srpm/SRPMS/`.

## 4. Submit build to COPR

```bash
just copr-build veighnsche/voxy v1.0.0-RC2
```

Equivalent manual command:

```bash
copr-cli build veighnsche/voxy target/rpm-srpm/SRPMS/*.src.rpm
```

To submit without waiting for completion:

```bash
VOXY_COPR_NOWAIT=1 just copr-build veighnsche/voxy v1.0.0-RC2
```

## 5. Install from COPR

On a Fedora client:

```bash
sudo dnf copr enable veighnsche/voxy
sudo dnf install -y voxy-app
```

## Notes

- COPR builds are source-based; release tags should be immutable.
- For pre-release tags like `1.0.0-RC2`, RPM version is normalized to `1.0.0~RC2` in SRPM metadata.
- Runtime behavior still depends on platform tray/compositor capabilities.
