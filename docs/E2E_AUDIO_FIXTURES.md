# E2E Audio Fixtures

## Purpose
Provide a deterministic local audio sample for STT e2e checks.

## Manifest
`tests/fixtures/audio/manifest.json` defines:
- source URL
- output filename
- SHA256 checksum
- expected transcript substring
- audio format metadata

## Commands
- Fetch fixture: `just fixtures-fetch`
- Verify fixture: `just fixtures-verify`
- Opt-in live e2e preflight: `just e2e-stt-live`

## Notes
- Live e2e remains opt-in (`VOXY_E2E_LIVE=1`).
- Fixture checks validate checksum before use.
