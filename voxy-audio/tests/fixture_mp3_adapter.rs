use voxy_audio::{fixtures::resolver, AudioFrameSource, FixtureMp3Adapter};

#[test]
fn fixture_mp3_adapter_decodes_test_3_fixture() {
    let fixture_path = resolver::resolve_fixture_mp3(&resolver::default_fixture_root(), "test_3")
        .expect("test_3 fixture path should resolve");

    let adapter = FixtureMp3Adapter::load(&fixture_path).expect("fixture should decode");

    assert!(adapter.sample_rate_hz() > 0);
    assert!(adapter.channels() > 0);
    assert!(adapter.remaining_frames().expect("remaining_frames") > 0);

    let first = adapter.read_frame().expect("at least one pcm frame");
    assert_eq!(first.sample_rate_hz, adapter.sample_rate_hz());
    assert_eq!(first.channels, adapter.channels());
    assert!(!first.is_empty());
}
