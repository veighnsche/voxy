# Publishing Checklist

This checklist is for getting `voxy-app` ready for distribution across:
- Flathub (Flatpak)
- AUR (Arch User Repository)
- COPR (Fedora)

Use this in addition to [../RELEASE.md](../RELEASE.md).

## 1. Current State (Repo Audit)

Already present:
- [x] MIT license: `LICENSE`
- [x] Changelog: `CHANGELOG.md`
- [x] Basic release checklist: `RELEASE.md`
- [x] CI checks for format/check/test/core + `voxy-app` check: `.github/workflows/ci.yml`
- [x] Local RPM build script: `scripts/release/build-rpm.sh`

Missing for marketplace publishing:
- [ ] Canonical, stable application ID decision for all channels
- [ ] Tracked desktop entry file in repo (not only generated in scripts)
- [ ] AppStream MetaInfo file (`*.metainfo.xml`)
- [ ] App icons in standard sizes (and ideally SVG)
- [ ] Flatpak manifest + permissions review
- [ ] AUR PKGBUILD + `.SRCINFO`
- [ ] COPR project + maintained `.spec` workflow from source tarball/SRPM

## 2. One-Time Decisions

- [ ] Choose and freeze app ID.
  - Recommendation: `io.github.veighnsche.Voxy` or `io.github.veighnsche.voxy`
  - Reason: easier verification path on Flathub for GitHub-hosted projects.
- [ ] Choose package naming policy:
  - AUR: `voxy-app` (source), `voxy-app-bin` (prebuilt), and/or `voxy-app-git` (VCS)
  - COPR project name (for example `veighnsche/voxy`)
- [ ] Define minimum supported distro/runtime matrix.

## 3. Common Release Gate (Run Every Release)

- [ ] Update `CHANGELOG.md` for the release.
- [ ] Ensure version/tag consistency.
- [ ] Run validation locally:
  - `cargo fmt --all -- --check`
  - `cargo check`
  - `cargo test -p voxy-core -p voxy-audio -p voxy-stt`
  - `cargo check -p voxy-app`
- [ ] Confirm CI green on `main`.
- [ ] Create and push signed tag `vX.Y.Z`.
- [ ] Draft GitHub release notes.

## 4. Metadata and Desktop Integration (Required Before Stores)

- [ ] Add tracked desktop file:
  - `packaging/linux/<app-id>.desktop`
- [ ] Add tracked AppStream file:
  - `packaging/linux/<app-id>.metainfo.xml`
- [ ] Add icons:
  - `packaging/linux/icons/hicolor/128x128/apps/<app-id>.png`
  - `packaging/linux/icons/hicolor/scalable/apps/<app-id>.svg` (preferred)
- [ ] Validate metadata:
  - `desktop-file-validate packaging/linux/<app-id>.desktop`
  - `appstreamcli validate packaging/linux/<app-id>.metainfo.xml`

## 5. Flatpak + Flathub Checklist

### 5.1 Local Flatpak Packaging

- [ ] Create manifest in repo:
  - `packaging/flatpak/<app-id>.yml`
- [ ] Use minimal permissions first; document each static permission.
- [ ] Ensure Rust dependencies are pinned reproducibly in manifest flow.
- [ ] Build locally:
  - `flatpak-builder --force-clean --user --install-deps-from=flathub --install build-flatpak packaging/flatpak/<app-id>.yml`
- [ ] Run app locally:
  - `flatpak run <app-id>`

### 5.2 Flathub Submission

- [ ] Read Flathub requirements and submission flow.
- [ ] Fork `flathub/flathub`, branch from `new-pr`, add manifest + metadata.
- [ ] Open PR to `new-pr` branch (not `master`).
- [ ] Address review comments; request test build.
- [ ] After merge, complete app verification in Flathub Developer Portal.

## 6. AUR Checklist

- [ ] Create `PKGBUILD` in a packaging repo/worktree.
- [ ] Generate `.SRCINFO`:
  - `makepkg --printsrcinfo > .SRCINFO`
- [ ] Validate package recipe:
  - `makepkg -s --cleanbuild`
  - `namcap PKGBUILD`
  - `namcap *.pkg.tar.*`
- [ ] Configure AUR SSH key in account profile.
- [ ] Publish package repo:
  - `git clone ssh://aur@aur.archlinux.org/<pkgbase>.git`
  - copy `PKGBUILD` + `.SRCINFO`
  - commit and `git push`
- [ ] Verify install on clean Arch environment.

## 7. COPR Checklist

- [ ] Maintain a `.spec` file in repo (recommended: `packaging/rpm/voxy-app.spec`).
- [ ] Build SRPM from tag/release source.
- [ ] Create COPR project and enable chroots.
- [ ] Submit build via CLI:
  - `copr-cli create ...`
  - `copr-cli build <owner>/<project> <package.src.rpm>`
- [ ] Test install path on Fedora:
  - `sudo dnf copr enable <owner>/<project>`
  - `sudo dnf install voxy-app`
- [ ] Document support policy for Fedora versions/chroots.

## 8. Recommended Rollout Order

- [ ] First: Flathub (best cross-distro UX and easiest user install story)
- [ ] Second: AUR (`-git` first, then stable package)
- [ ] Third: COPR (RPM consumers)

## 9. Post-Release Operations

- [ ] Monitor issues/crash reports for 48 hours.
- [ ] Verify update path from previous version on each channel.
- [ ] Announce release notes and known limitations.

## References

- Flathub submission: https://docs.flathub.org/docs/for-app-authors/submission
- Flathub requirements: https://docs.flathub.org/docs/for-app-authors/requirements
- Flathub MetaInfo guidelines: https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines
- Flathub verification: https://docs.flathub.org/docs/for-app-authors/verification
- Flatpak reference docs: https://docs.flatpak.org/en/latest/command-references.html
- COPR user documentation: https://docs.pagure.org/copr.copr/user_documentation.html
- COPR CLI quickstart/examples: https://developer.fedoraproject.org/deployment/copr/copr-cli.html
- AUR submission guidelines: https://wiki.archlinux.org/title/AUR_submission_guidelines
- PKGBUILD format: https://man.archlinux.org/man/PKGBUILD.5.en
- makepkg: https://man.archlinux.org/man/makepkg.8.en
- namcap: https://man.archlinux.org/man/extra/namcap/namcap.1.en
- desktop-file-validate: https://manpages.debian.org/testing/desktop-file-utils/desktop-file-validate.1.en.html
- appstreamcli: https://www.freedesktop.org/software/appstream/docs/re01.html
