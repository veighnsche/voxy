use voxy_audio::{AudioError, AudioFrameSource, AudioInput, AudioRoute, InputEngine};

fn start_microphone_or_skip(engine: &InputEngine) -> bool {
    match engine.start_checked() {
        Ok(()) => true,
        Err(
            error @ (AudioError::CpalNoInputDevice
            | AudioError::CpalDefaultInputConfig(_)
            | AudioError::CpalBuildStream(_)
            | AudioError::CpalPlayStream(_)),
        ) => {
            eprintln!("skipping audio integration assertion due to host setup: {error}");
            false
        }
        Err(error) => panic!("unexpected start error: {error}"),
    }
}

#[test]
fn microphone_start_provides_frames() {
    let engine = InputEngine::new();

    if !start_microphone_or_skip(&engine) {
        return;
    }
    assert_eq!(engine.current_route(), AudioRoute::Microphone);
    let sample_rate_hz = engine.sample_rate_hz();
    let channels = engine.channels();
    assert!(
        (8_000..=192_000).contains(&sample_rate_hz),
        "unexpected sample rate from microphone source: {sample_rate_hz}"
    );
    assert!(
        (1..=16).contains(&channels),
        "unexpected channel count from microphone source: {channels}"
    );

    let snapshot = engine.snapshot().expect("snapshot should be available");
    assert!(snapshot.running);
    assert_eq!(snapshot.route, AudioRoute::Microphone);

    let _ = engine.read_frame();

    engine.stop();
    let snapshot = engine.snapshot().expect("snapshot should still work");
    assert!(!snapshot.running);
    assert!(engine.read_frame().is_none());
}

#[test]
fn set_route_microphone_while_running_keeps_session_route() {
    let engine = InputEngine::new();
    if !start_microphone_or_skip(&engine) {
        return;
    }
    assert_eq!(engine.current_route(), AudioRoute::Microphone);

    engine
        .set_route_checked(AudioRoute::Microphone)
        .expect("setting microphone route should succeed");

    let snapshot = engine.snapshot().expect("snapshot should be available");
    assert!(snapshot.running);
    assert_eq!(snapshot.route, AudioRoute::Microphone);
}

#[test]
fn set_route_microphone_while_stopped_keeps_microphone_route() {
    let engine = InputEngine::new();
    engine
        .set_route_checked(AudioRoute::Microphone)
        .expect("setting microphone route should succeed");

    let snapshot = engine.snapshot().expect("snapshot should be available");
    assert!(!snapshot.running);
    assert_eq!(snapshot.route, AudioRoute::Microphone);

    if !start_microphone_or_skip(&engine) {
        return;
    }

    let snapshot = engine.snapshot().expect("snapshot should be available");
    assert!(snapshot.running);
    assert_eq!(snapshot.route, AudioRoute::Microphone);
    engine.stop();
}
