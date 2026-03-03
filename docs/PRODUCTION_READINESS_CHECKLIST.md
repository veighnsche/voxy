# Production Readiness Checklist

This checklist is the launch gate for Voxy releases.
Use it as a strict pass/fail list, not as guidance.
Actionable P0 implementation tickets live in `docs/PRODUCTION_P0_TICKETS.md`.

## Current Baseline (Audit Date: 2026-03-03)

Status: **NOT production ready**.

Highest current risks:
- Release evidence remains manual (CI links, rollback drill, owner assignment).

## P0: Must Pass Before Any Production Launch

### 1. User Safety and Exit Semantics
- [x] Closing window can always exit app when tray is unavailable.
- [x] Quit is available from UI even if status notifier fails.
- [x] Quit path has bounded shutdown time and does not hang.
- [ ] Required evidence: manual test matrix across tray-supported and tray-unsupported environments.
- [ ] Code refs: `voxy-app/src/app/behavior/visibility/close_request.rs`, `voxy-app/src/app/controller/bootstrap.rs`, `voxy-app/src/tray/status_notifier.rs`.

### 2. Lossless Critical Event Handling
- [x] `QuitRequested`, `HideRequested`, `MicToggled`, and `RuntimeError` are delivered losslessly.
- [x] Event send failures are surfaced with telemetry/logging, not silently ignored.
- [x] Channel saturation behavior is tested under stress.
- [x] Required evidence: saturation test + trace showing no dropped critical events.
- [ ] Code refs: `voxy-app/src/wiring/channels.rs`, `voxy-app/src/wiring/command_bus/mod.rs`, `voxy-app/src/tray/status_notifier.rs`.

### 3. Transcript Integrity on Stop
- [x] Stop waits for commit + completion (or deterministic timeout path) before close.
- [x] Final segment is not lost during stop, disconnect, or reconnect.
- [x] Protocol correlation uses item identifiers where applicable.
- [x] Required evidence: deterministic integration tests with mocked websocket server.
- [ ] Code refs: `voxy-stt/src/realtime/client.rs`, `voxy-stt/src/realtime/event_mapper.rs`, `voxy-stt/src/realtime/protocol/server_event.rs`.

### 4. Audio Reliability and Error Surfacing
- [x] Audio start/stop/read errors are never silently swallowed in production paths.
- [x] CPAL startup/teardown has timeout and clear failure reporting.
- [x] `VOXY_AUDIO_FRAME_MS` has strict bounded validation.
- [x] Frame/buffer sizing math is overflow-safe.
- [x] Required evidence: failure-injection tests for no-device/build/play/lock failure paths.
- [ ] Code refs: `voxy-audio/src/engine/input_engine.rs`, `voxy-audio/src/adapters/cpal/source.rs`, `voxy-audio/src/adapters/cpal/config.rs`, `voxy-audio/src/adapters/cpal/state.rs`.

### 5. Durable, Non-Blocking Settings Persistence
- [x] Settings writes are off the UI event loop and debounced.
- [x] Settings write path is atomic and crash-safe (`tmp` + rename + durability step).
- [x] Persistence failures are visible in UI (not trace-only).
- [x] Required evidence: crash/kill simulation showing previous-good or new-good settings file.
- [ ] Code refs: `voxy-app/src/app/controller/event_processing.rs`, `voxy-app/src/app/controller/settings_sync.rs`, `voxy-app/src/app/settings_store/file_store.rs`.

### 6. CI and Quality Gates
- [x] CI enforces `cargo fmt --all -- --check`.
- [x] CI enforces `cargo check --workspace`.
- [x] CI enforces `cargo test -p voxy-core -p voxy-audio -p voxy-stt -p voxy-app`.
- [x] CI enforces `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] CI enforces required GUI smoke/lifecycle flows via `xtask`.
- [ ] Required evidence: green CI run link on release commit/tag.
- [ ] Code refs: `.github/workflows/ci.yml`, `justfile`, `xtask/src/tasks/gui`.

### 7. End-to-End Coverage
- [x] Fixture-based e2e tests are implemented and run in CI.
- [x] Live STT e2e remains opt-in but is runnable in controlled release validation.
- [ ] Required evidence: e2e test logs attached to release evidence bundle.
- [ ] Code refs: `tests/e2e/stt_fixture_smoke.rs`, `tests/e2e/stt_live_opt_in.rs`.

### 8. Release Provenance and Rollback
- [x] Tag-triggered release workflow builds artifacts from clean checkout.
- [x] Checksums are published for release artifacts.
- [ ] Rollback rehearsal is executed and documented.
- [ ] Release owner and first-48h on-call owner are assigned.
- [ ] Required evidence: release workflow URL, checksum manifest, rollback drill notes.
- [ ] Code refs: `RELEASE.md`, `docs/OPERATIONS.md`, `scripts/release/build-rpm.sh`.

## P1: Required Soon After P0 (Hardening)

### 9. STT Session and Retry Hygiene
- [x] Worker lifecycle self-recovers if background task exits unexpectedly.
- [x] Retry strategy includes jitter and bounded defaults.
- [x] Retryability matrix avoids retrying clearly permanent classes.
- [x] Unknown/malformed server payloads emit diagnostics.
- [ ] Code refs: `voxy-stt/src/realtime/client.rs`, `voxy-stt/src/realtime/backoff.rs`.

### 10. Security and Secrets
- [x] Trace output redacts sensitive URL/query/path metadata.
- [x] Dotenv loading behavior is explicit and bounded to intended locations.
- [x] CI includes dependency vulnerability and policy scanning.
- [ ] Code refs: `voxy-stt/src/config.rs`, `voxy-stt/src/realtime/client.rs`, `.github/workflows/ci.yml`.

### 11. Core Domain Contract Consistency
- [x] State transition contract is consistent between exported transition logic and reducer behavior.
- [x] NaN/inf inputs are sanitized before gate policy comparisons.
- [ ] Code refs: `voxy-core/src/state.rs`, `voxy-core/src/model/mod.rs`, `voxy-core/src/config.rs`, `voxy-core/src/recording_stop/mod.rs`.

## P2: Production Maturity (Recommended)

### 12. Performance and Soak
- [ ] 30+ minute soak test with real mic + network fault injection passes.
- [ ] Idle/active CPU and memory budgets are defined and measured.
- [ ] Audio-to-text latency SLO is defined and tracked.

### 13. Distribution and Packaging
- [x] Packaging validation checks are automated (`desktop-file-validate`, appstream validation, install smoke).
- [x] Release artifacts and distribution metadata are fully reviewable in source control.

## Release Evidence Bundle (Attach to Every Launch)

- [ ] Commit/tag being released.
- [ ] CI run URL for all mandatory gates.
- [ ] Manual QA notes (real microphone + real STT backend).
- [ ] e2e output (fixture mandatory, live opt-in if run).
- [ ] Artifact list + checksums.
- [ ] Rollback target and rollback drill date.
- [ ] Known issues with owner and target fix date.
- [ ] Explicit go/no-go sign-off (name + timestamp).
