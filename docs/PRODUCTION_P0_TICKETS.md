# Production P0 Tickets

These tickets turn the P0 section of `docs/PRODUCTION_READINESS_CHECKLIST.md` into implementation work items with acceptance criteria.

## VOXY-P0-001: Tray-Unavailable Quit Fallback
Status: `implemented in code/tests`; manual matrix verification pending.

### Goal
Ensure users can always exit the app when tray/status notifier is unavailable.

### Acceptance Criteria
- When tray startup fails, window close request exits the app instead of hiding it.
- When tray startup fails, the in-app close button triggers quit instead of visibility toggle.
- When tray is available, existing hide-to-tray behavior is unchanged.
- A runtime error is still surfaced to users when tray initialization fails.

### Verification
- Manual: run with tray disabled/unavailable and confirm close exits.
- Manual: run with tray available and confirm close hides.
- Automated: `cargo run -p xtask -- gui quit-fallback` asserts tray-disabled close exits within timeout.

## VOXY-P0-002: Lossless Critical Event Delivery
Status: `implemented in code/tests`; telemetry dashboarding still optional.

### Goal
Guarantee delivery for critical lifecycle events even under channel pressure.

### Acceptance Criteria
- `QuitRequested`, `HideRequested`, `MicToggled`, `RuntimeError` are no longer lossy.
- Event-send failure paths are observable (logged or surfaced with metrics counters).
- Channel-saturation test demonstrates critical events are still processed.

### Verification
- Automated saturation unit test: `wiring::event_emit::tests::critical_emit_retries_when_channel_is_full`.
- Validation evidence: `just validate` includes this test path.

## VOXY-P0-003: Deterministic Stop/Commit Transcript Flush
Status: `partially implemented`; reconnect edge-case integration coverage still pending.

### Goal
Prevent final transcript loss during stop and shutdown.

### Acceptance Criteria
- Stop path waits for commit acknowledgement and completion event, with timeout fallback.
- On timeout/failure, user-visible error indicates partial transcript risk.
- Final segment integrity is preserved across normal stop and reconnect scenarios.

### Verification
- Deterministic payload/sequence tests cover matching completion, mismatched completion suppression, and stale-completion rejection until fresh commit-ack.
- Remaining gap: full mocked-websocket reconnect scenario coverage.

## VOXY-P0-004: Audio Failure Surface and Bounds Safety
Status: `implemented in code/tests`.

### Goal
Eliminate silent audio failures and unsafe frame-size configuration.

### Acceptance Criteria
- Audio start/stop/read errors are surfaced (not dropped).
- CPAL worker startup and shutdown are bounded by timeout and return actionable errors.
- `VOXY_AUDIO_FRAME_MS` has enforced min/max bounds.
- Frame and buffer sizing use overflow-safe arithmetic with explicit error handling.

### Verification
- Unit/integration tests cover no-input-device, build/play failure, startup timeout/disconnect mapping, lock poison, and bad env inputs.
- Manual run with invalid config confirms explicit runtime errors.

## VOXY-P0-005: Atomic and Non-Blocking Settings Persistence
Status: `implemented in code`; crash-kill durability proof pending.

### Goal
Make settings persistence resilient and non-blocking for the UI loop.

### Acceptance Criteria
- Settings saves are moved off main GTK event loop or batched/debounced.
- Settings file writes are atomic (`tmp` + rename) and durable.
- Save/load errors are shown in UI (not trace-only).

### Verification
- Crash/kill simulation during write keeps either old-good or new-good file.
- UI interaction remains responsive during rapid settings changes.

## VOXY-P0-006: CI Gate Expansion
Status: `implemented in workflows`; requires first green CI evidence link.

### Goal
Align CI with production readiness requirements.

### Acceptance Criteria
- CI runs and enforces:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
  - `cargo test -p voxy-core -p voxy-audio -p voxy-stt -p voxy-app`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - required `xtask` GUI smoke/lifecycle flows
- Failing any gate blocks merge/release.

### Verification
- Green CI run on target commit includes all required jobs.

## VOXY-P0-007: Real E2E Harness in CI
Status: `implemented`; fixture CI flow + opt-in live harness are both available.

### Goal
Replace placeholder e2e specs with executable coverage.

### Acceptance Criteria
- `tests/e2e/stt_fixture_smoke.rs` contains executable fixture e2e test(s).
- Fixture e2e runs in CI and is required.
- Live e2e remains opt-in but can be executed in release validation with documented env/secrets.

### Verification
- CI logs show fixture e2e execution and pass/fail.
- Live opt-in command: `VOXY_E2E_LIVE_STT=1 cargo run -p xtask -- gui stt-live-opt-in`.

## VOXY-P0-008: Release Provenance, Integrity, and Rollback Drill
Status: `partially implemented`; release workflow + checksums added, rollback drill still pending.

### Goal
Make release process reproducible, verifiable, and rollback-ready.

### Acceptance Criteria
- Tag-triggered release workflow builds artifacts from clean checkout.
- Release artifacts publish checksums.
- Rollback rehearsal is documented before production launch.
- Release owner and first-48h on-call owner are recorded per release.

### Verification
- Release workflow URL and checksum manifest attached to release.
- Rollback drill notes linked in release evidence.
