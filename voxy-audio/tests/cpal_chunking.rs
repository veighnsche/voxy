use voxy_audio::adapters::cpal::state::CaptureBuffer;

#[test]
fn capture_buffer_outputs_20ms_frames_for_16k_mono() {
    let buffer = CaptureBuffer::new(16_000, 1);

    // 20ms @ 16k mono => 320 samples per frame
    let first_half = vec![7i16; 160];
    let second_half = vec![9i16; 160];

    buffer
        .push_i16_samples(&first_half)
        .expect("push should succeed");
    assert!(buffer.pop_frame().expect("pop should succeed").is_none());

    buffer
        .push_i16_samples(&second_half)
        .expect("push should succeed");
    let frame = buffer
        .pop_frame()
        .expect("pop should succeed")
        .expect("frame should be ready");

    assert_eq!(frame.sample_rate_hz, 16_000);
    assert_eq!(frame.channels, 1);
    assert_eq!(frame.samples_i16.len(), 320);
}

#[test]
fn capture_buffer_outputs_20ms_frames_for_48k_stereo() {
    let buffer = CaptureBuffer::new(48_000, 2);

    // 20ms @ 48k stereo => 1920 interleaved samples
    let mut samples = Vec::with_capacity(1_920);
    for idx in 0..1_920 {
        samples.push((idx % i16::MAX as usize) as i16);
    }

    buffer
        .push_i16_samples(&samples)
        .expect("push should succeed");
    let frame = buffer
        .pop_frame()
        .expect("pop should succeed")
        .expect("frame should be ready");

    assert_eq!(frame.sample_rate_hz, 48_000);
    assert_eq!(frame.channels, 2);
    assert_eq!(frame.samples_i16.len(), 1_920);
}
