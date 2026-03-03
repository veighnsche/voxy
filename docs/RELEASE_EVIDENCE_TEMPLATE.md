# Release Evidence Template

Copy this template into release notes (or issue tracker) for every production launch.
For a pre-filled scaffold, run:
- `just release-evidence version=<X.Y.Z[-RCN]>`

## Release Identity

- Release tag:
- Release commit:
- Release owner:
- 48h on-call owner:
- Date/time (UTC):

## Required Gate Evidence

- CI checks URL:
- Security scan URL:
- Release workflow URL:
- Packaging validation evidence:

## Runtime Validation

- Fixture e2e output:
- Live e2e output (if run):
- Manual QA notes (mic + backend):

## Artifact Integrity

- Artifact list:
  - `voxy-app-<version>-linux-x86_64.tar.gz`
  - `screenshots/idle.png`
  - `screenshots/recording.png`
  - `screenshots/config.png`
- SHA256SUMS reference:
- Local checksum verification result:

## Rollback Readiness

- Last known good tag:
- Rollback rehearsal date:
- Rollback rehearsal operator:
- Rollback rehearsal notes:

## Decision

- Known issues:
- Go / No-Go:
- Approver:
