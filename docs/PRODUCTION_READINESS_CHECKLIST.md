# Production Readiness Checklist

Use this checklist before calling a Voxy release "production ready."

Production-ready means:
- critical items are checked
- no known data-loss or security blocker remains
- runbook + rollback are documented and tested

## Audit Snapshot (2026-03-02)

Legend:
- `[x]` verified in this audit pass (command output and/or code inspection)
- `[ ]` not yet verified, failing, or not yet decided

Commands executed during this snapshot:
- `just validate` -> passed
- `cargo build --release -p voxy-app` -> passed
- `cargo run -p xtask -- gui smoke` -> passed
- `cargo run -p xtask -- gui visibility-window-guard` -> passed
- `cargo test -p voxy-core` -> passed (`28 passed`)
- `cargo test -p voxy-audio` -> passed (`16 passed`)
- `cargo test -p voxy-stt` -> passed (`26 passed`)
- `cargo test -p voxy-app` -> passed (`29 passed`)
- `just make rpm package` -> passed
- `rpm -qlp target/rpm/RPMS/x86_64/voxy-app-0.1.0-1.um43.x86_64.rpm` -> includes desktop file, SVG icon, metainfo
- `HOME=$(mktemp -d) ./scripts/dev/install-desktop.sh` -> passed

Artifacts produced:
- RPM: `target/rpm/RPMS/x86_64/voxy-app-0.1.0-1.um43.x86_64.rpm`
- Desktop install smoke (temp HOME):
  - `.local/bin/voxy-app`
  - `.local/share/applications/com.vince.voxy.desktop`

## 1. Release Scope and Ownership

- [ ] Release owner is assigned for this version.
- [ ] Scope is frozen (features, fixes, known exclusions).
- [ ] Known risks are documented and explicitly accepted.
- [ ] Version/tag plan is decided (`vX.Y.Z`) for this release.

## 2. Build, CI, and Quality Gates

- [x] `just validate` passes locally.
- [ ] CI is green on `main`.
- [x] CI includes at least:
  - `cargo fmt --all -- --check`
  - `cargo check`
  - `cargo test -p voxy-core -p voxy-audio -p voxy-stt`
  - `cargo clippy --all-targets -- -D warnings`
  - GUI smoke flow(s) via `xtask`
- [x] Release build command is repeatable (`cargo build --release -p voxy-app`).

## 3. Functional Correctness (App Flows)

- [ ] Recording start/stop works with a real microphone.
- [ ] Live deltas appear while recording (real STT backend).
- [ ] Commit behavior is correct on stop (no missing final text segment).
- [ ] User edit behavior is correct (manual edits do not corrupt committed/live merge).
- [ ] Copy/Reset actions behave correctly in manual QA.
- [ ] Model switching works (`gpt-4o-mini-transcribe`, `gpt-4o-transcribe`).
- [x] Window visibility flow works (show/hide, no extra window instance).
- [ ] Tray menu actions all work on supported environments.

## 4. Configuration and Persistence

- [x] Settings persistence works across app restarts:
  - `silence_auto_stop_seconds`
  - `silence_gate_threshold`
  - `vad_silence_ms`
- [x] Settings file path behavior is implemented:
  - `$XDG_CONFIG_HOME/voxy/settings.json`
  - fallback `~/.config/voxy/settings.json`
- [x] Startup behavior is validated when config file is missing, malformed, or partially populated.
- [x] Runtime env vars are documented and validated (`docs/DEV_ENV.md` + README):
  - `VOXY_STT_BACKEND`
  - `VOXY_OPENAI_REALTIME_URL`
  - `VOXY_UI_EVENT_POLL_MS`
  - `VOXY_STT_SOURCE_POLL_MS`
  - `VOXY_AUDIO_FRAME_MS`
  - `VOXY_STT_VAD_SILENCE_MS`
  - `VOXY_SILENCE_AUTO_STOP_SECONDS`
  - `VOXY_MAX_RECORDING_SECONDS`

## 5. Security and Secrets Handling

- [x] API key ingestion order is implemented and matches `docs/API_KEY_INGESTION.md`.
- [x] API keys are not rendered in UI and key values are not logged.
- [x] Error paths reviewed for secret leakage (no key value surfaced in configured error strings).
- [x] `.env` / `.env.local` guidance is documented for users.
- [x] Security disclosure path is documented (`SECURITY.md`) and visible.

## 6. Reliability and Failure Handling

- [ ] Missing microphone device behavior is manually validated for clear runtime UX.
- [x] Invalid/missing OpenAI key yields explicit runtime error messaging.
- [x] Network disconnect behavior is defined and tested (recover or fail clearly).
- [x] Realtime websocket error handling is tested under injected fault conditions.
- [x] Shutdown is graceful in smoke flow (process exits cleanly after signal).
- [ ] Long-session stability test performed (>= 30 minutes).
- [x] Reconnect/backoff strategy is explicitly decided and applied (or removed).

## 7. Performance and Resource Use

- [ ] Audio-to-text latency is measured and within target.
- [ ] CPU and memory are profiled during active recording/transcription.
- [ ] Idle CPU usage is acceptable when not recording.
- [x] No unbounded buffer growth under sustained input.
- [x] VAD/silence defaults are tuned for practical sentence continuity.

## 8. Observability and Diagnostics

- [x] `VOXY_TRACE_PIPELINE` tracing is implemented for app/audio/stt.
- [x] Error report copy path exists and includes actionable diagnostics context.
- [x] Diagnostic logs reviewed to avoid direct secret value logging.
- [x] Support workflow is documented (how to collect traces/logs and where to file reports).

## 9. Platform and Packaging Readiness

- [x] Desktop launch/install path verified (`scripts/dev/install-desktop.sh`) for this release.
- [x] RPM build path verified (`scripts/release/build-rpm.sh`) for this release.
- [x] Publishing tasks are tracked via `docs/PUBLISHING.md`.
- [x] App metadata/assets are complete for target channels (desktop file, icons, appstream).

## 10. Operations (Runbook + Rollback)

- [x] Release runbook exists (`RELEASE.md`).
- [x] Rollback procedure is documented.
- [ ] Rollback procedure is tested.
- [x] Known issue triage workflow is documented.
- [ ] Ownership for post-release monitoring window is assigned.

## 11. Launch Decision

- [ ] All critical blockers are closed.
- [ ] Remaining non-critical issues are explicitly deferred with owners/dates.
- [ ] Final go/no-go sign-off recorded.

## Evidence to Attach per Release

- [ ] CI run link.
- [x] `just validate` output summary captured.
- [ ] Manual test notes for recording/transcription flows.
- [x] Packaging artifact references (RPM / installers / release bundle).
- [ ] Release notes + changelog link.
