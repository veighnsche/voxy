# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog,
and this project adheres to Semantic Versioning.

## [Unreleased]

## [1.0.0-RC2] - 2026-03-03
### Added
- COPR publishing workflow/docs/`just` targets, including SRPM build and submit helpers.
- Release metadata preflight and release-evidence scaffold generation commands.
- Release artifacts now include canonical app screenshots with checksum manifests.

### Changed
- Split `voxy-stt` realtime client into focused modules to reduce single-file complexity.
- Refined AppStream screenshot metadata and README screenshot gallery for packaging visibility.

### Fixed
- Suppressed benign stop-flush empty-buffer errors to avoid false runtime failures after silence timeout.
- Hardened stop error matcher coverage for empty-buffer variant formats.

## [1.0.0-RC1] - 2026-03-03
### Added
- Initial workspace scaffold (`voxy-app`, `voxy-core`, `voxy-stt`, `voxy-audio`)
- Core reducer model with command-driven side effects
- Architecture and planning documents
- Repository publishing/readiness files (CI, templates, contribution docs)
