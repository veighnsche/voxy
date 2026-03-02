use std::time::{Duration, Instant};

use super::{RecordingStopPolicyState, RecordingStopReason};

#[test]
fn silence_policy_emits_countdown_and_then_stops() {
    let mut state = RecordingStopPolicyState::default();
    let base = Instant::now();

    let first = state.evaluate(base, true, 0.0, 2, 0.3, None);
    assert_eq!(first.silence_seconds_remaining, Some(2));
    assert_eq!(first.stop_reason, None);

    let second = state.evaluate(base + Duration::from_secs(1), true, 0.0, 2, 0.3, None);
    assert_eq!(second.silence_seconds_remaining, Some(1));
    assert_eq!(second.stop_reason, None);

    let third = state.evaluate(base + Duration::from_secs(2), true, 0.0, 2, 0.3, None);
    assert_eq!(
        third.stop_reason,
        Some(RecordingStopReason::SilenceAutoStop)
    );
    assert_eq!(third.silence_seconds_remaining, Some(0));
}

#[test]
fn silence_policy_resets_after_sustained_above_threshold_level() {
    let mut state = RecordingStopPolicyState::default();
    let base = Instant::now();

    let _ = state.evaluate(base, true, 0.0, 3, 0.3, None);
    let _ = state.evaluate(base + Duration::from_millis(100), true, 1.0, 3, 0.3, None);
    let decision = state.evaluate(base + Duration::from_millis(450), true, 1.0, 3, 0.3, None);

    assert_eq!(decision.stop_reason, None);
    assert_eq!(decision.silence_seconds_remaining, None);
}

#[test]
fn max_duration_emits_stop_reason_once() {
    let mut state = RecordingStopPolicyState::default();
    let base = Instant::now();
    let max = Some(Duration::from_secs(3));

    let _ = state.evaluate(base, true, 0.9, 0, 0.3, max);

    let trigger = state.evaluate(base + Duration::from_secs(3), true, 0.9, 0, 0.3, max);
    assert_eq!(
        trigger.stop_reason,
        Some(RecordingStopReason::MaxRecordingDuration)
    );

    let repeated = state.evaluate(base + Duration::from_secs(4), true, 0.9, 0, 0.3, max);
    assert_eq!(repeated.stop_reason, None);
}

#[test]
fn policy_state_resets_when_recording_is_not_active() {
    let mut state = RecordingStopPolicyState::default();
    let base = Instant::now();

    let _ = state.evaluate(base, true, 0.0, 2, 0.3, None);
    let idle = state.evaluate(base + Duration::from_secs(1), false, 0.0, 2, 0.3, None);

    assert_eq!(idle.stop_reason, None);
    assert_eq!(idle.silence_seconds_remaining, None);

    let fresh = state.evaluate(base + Duration::from_secs(2), true, 0.0, 2, 0.3, None);
    assert_eq!(fresh.silence_seconds_remaining, Some(2));
}
