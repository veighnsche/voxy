# Operations Guide

This runbook covers support intake, incident triage, diagnostics collection, and rollback.

## 1. Ownership Per Release

Before release:
- assign a release owner
- assign an on-call owner for the first 48h after release
- record both names in the release notes

## 2. Support Intake

When a user reports a bug, request:
- app version / git commit
- OS + compositor/session details
- exact reproduction steps
- expected vs actual behavior
- whether issue is deterministic or intermittent

Never request raw API keys.

## 3. Diagnostics Collection

## 3.1 Enable Pipeline Trace

Run with tracing:

```bash
VOXY_TRACE_PIPELINE=1 just gui
```

Optional verbosity controls:

```bash
VOXY_TRACE_PIPELINE=1 \
VOXY_TRACE_PIPELINE_EVERY=10 \
VOXY_TRACE_PIPELINE_NOISY_EVERY=200 \
just gui
```

Capture both stdout/stderr and attach them to the issue.

## 3.2 Capture Error Report

When the runtime error banner appears:
- click `Copy Error`
- paste the payload into the issue/advisory

The report includes state, window, and buffer context without exposing API key values.

## 4. Triage Workflow

Use this sequence:
1. Reproduce locally with the same env vars/settings.
2. Classify severity:
   - `S1`: data loss/security/privacy crash loop
   - `S2`: core feature broken with workaround
   - `S3`: non-critical bug or UX regression
3. Confirm blast radius:
   - backend-specific (`dummy` vs `openai_api`)
   - compositor-specific
   - device-specific (audio hardware/sample rate)
4. Identify owner and ETA.
5. Add mitigation/workaround notes to release/issue tracker.

## 5. Rollback Procedure

## 5.1 Source Rollback (GitHub Release)

If a release is bad:
1. Mark release as withdrawn in notes/changelog.
2. Revert offending commit(s) or cherry-pick a hotfix onto `main`.
3. Run:
   - `just validate`
   - `cargo build --release -p voxy-app`
4. Tag and publish patched release (`vX.Y.(Z+1)`).

Never rewrite published tags.

## 5.2 RPM Rollback

For local RPM artifacts:
1. Rebuild a known-good commit/tag:
   - `just make rpm package`
2. Replace the deployed RPM with the known-good artifact.
3. Verify startup and smoke flow on target machine.

## 5.3 Desktop Install Rollback

If installed via `scripts/dev/install-desktop.sh`:
1. Reinstall known-good binary from a known-good commit.
2. Verify:
   - `~/.local/bin/voxy-app`
   - `~/.local/share/applications/com.vince.voxy.desktop`
3. Launch app and run a quick recording smoke check.

## 6. Post-Release Monitoring (First 48h)

Track:
- startup failures
- microphone start failures
- websocket/auth failures
- commit/final-segment regressions
- visibility/window lifecycle regressions

If S1/S2 spike appears, pause promotion and execute rollback.
