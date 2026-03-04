# Release Evidence Records

Store generated release evidence files in this directory.
These files are point-in-time scaffolds, not immutable release records.

Example:
- `just release-evidence version=1.0.0-RC3`

This creates:
- `docs/release-evidence/v1.0.0-RC3.md`

Attach the file content to your GitHub release notes or issue tracker,
then fill in the remaining manual fields (owner, on-call, QA notes,
rollback drill details, and go/no-go sign-off).
Regenerate the scaffold right before cutting/publishing a release tag.
