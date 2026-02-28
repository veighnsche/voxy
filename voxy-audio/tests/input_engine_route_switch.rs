use voxy_audio::{AudioFrameSource, AudioInput, AudioRoute, InputEngine};

#[test]
fn input_engine_can_switch_from_mic_to_fixture_while_running() {
    let engine = InputEngine::new();

    engine.start();
    assert_eq!(engine.current_route(), AudioRoute::Microphone);
    assert_eq!(engine.sample_rate_hz(), 16_000);
    assert_eq!(engine.channels(), 1);

    engine
        .set_route(AudioRoute::fixture_test(3))
        .expect("route switch to test_3 should succeed");

    let snapshot = engine.snapshot().expect("snapshot should be available");
    assert!(snapshot.running);
    assert_eq!(snapshot.route, AudioRoute::fixture_test(3));

    let frame = engine
        .read_frame()
        .expect("fixture route should provide decoded frames");
    assert!(!frame.is_empty());

    engine.stop();
    let snapshot = engine.snapshot().expect("snapshot should still work");
    assert!(!snapshot.running);
    assert!(engine.read_frame().is_none());
}

#[test]
fn set_route_failure_while_running_does_not_change_session_route() {
    let engine = InputEngine::new();
    engine
        .start_checked()
        .expect("microphone route should start");
    assert_eq!(engine.current_route(), AudioRoute::Microphone);

    let result = engine.set_route_checked(AudioRoute::fixture_test(99));
    assert!(result.is_err());

    let snapshot = engine.snapshot().expect("snapshot should be available");
    assert!(snapshot.running);
    assert_eq!(snapshot.route, AudioRoute::Microphone);
}

#[test]
fn start_failure_does_not_mark_session_running() {
    let engine = InputEngine::new();
    engine
        .set_route_checked(AudioRoute::fixture_test(99))
        .expect("setting route while stopped should not fail");

    let result = engine.start_checked();
    assert!(result.is_err());

    let snapshot = engine.snapshot().expect("snapshot should be available");
    assert!(!snapshot.running);
    assert_eq!(snapshot.route, AudioRoute::fixture_test(99));
}
