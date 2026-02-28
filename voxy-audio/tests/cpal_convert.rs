use voxy_audio::adapters::cpal::convert::{
    convert_f32_buffer, convert_u16_buffer, f32_to_i16, u16_to_i16,
};

#[test]
fn f32_scalar_conversion_clamps_and_scales() {
    assert_eq!(f32_to_i16(0.0), 0);
    assert_eq!(f32_to_i16(1.0), i16::MAX);
    assert_eq!(f32_to_i16(-1.0), i16::MIN + 1);
    assert_eq!(f32_to_i16(2.0), i16::MAX);
    assert_eq!(f32_to_i16(-2.0), i16::MIN + 1);
}

#[test]
fn u16_scalar_conversion_maps_midpoint_to_zero() {
    assert_eq!(u16_to_i16(0), i16::MIN);
    assert_eq!(u16_to_i16(32_768), 0);
    assert_eq!(u16_to_i16(u16::MAX), i16::MAX);
}

#[test]
fn f32_vector_conversion_preserves_shape() {
    let samples = [-1.0, -0.5, 0.0, 0.5, 1.0];
    let converted = convert_f32_buffer(&samples);
    assert_eq!(converted.len(), samples.len());
    assert_eq!(converted[0], i16::MIN + 1);
    assert_eq!(converted[2], 0);
    assert_eq!(converted[4], i16::MAX);
}

#[test]
fn u16_vector_conversion_preserves_shape() {
    let samples = [0, 1000, 32_768, 60_000, u16::MAX];
    let converted = convert_u16_buffer(&samples);
    assert_eq!(converted.len(), samples.len());
    assert_eq!(converted[0], i16::MIN);
    assert_eq!(converted[2], 0);
    assert_eq!(converted[4], i16::MAX);
}
