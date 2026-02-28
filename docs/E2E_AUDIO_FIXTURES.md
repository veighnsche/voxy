# E2E Audio Fixtures

## Purpose
Use local, user-managed audio fixtures for STT e2e checks.

## Fixture Set
The required fixture pairs are:
- `tests/fixtures/audio/test_1.mp3` + `tests/fixtures/audio/test_1.txt`
- `tests/fixtures/audio/test_2.mp3` + `tests/fixtures/audio/test_2.txt`
- `tests/fixtures/audio/test_3.mp3` + `tests/fixtures/audio/test_3.txt`
- `tests/fixtures/audio/test_4.mp3` + `tests/fixtures/audio/test_4.txt`
- `tests/fixtures/audio/test_5.mp3` + `tests/fixtures/audio/test_5.txt`

## Catalog
- Use `just fixtures-list` to print available local pairs and transcript previews.

## Commands
- List fixtures: `just fixtures-list`
- Opt-in live e2e preflight: `just e2e-stt-live`
- Single fixture override: `VOXY_E2E_LIVE=1 cargo run -p xtask -- gui stt-e2e --fixture-id 3`

## Notes
- Live e2e remains opt-in (`VOXY_E2E_LIVE=1`).
- No download/fetch step is used.
- `xtask gui stt-e2e` writes transcription output to `tests/fixtures/audio/test_<id>.result.txt`.
- Default behavior runs all fixtures (`test_1` through `test_5`).
