pub fn f32_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32).round() as i16
}

pub fn u16_to_i16(sample: u16) -> i16 {
    (sample as i32 - 32_768) as i16
}

pub fn convert_f32_buffer(samples: &[f32]) -> Vec<i16> {
    samples.iter().copied().map(f32_to_i16).collect()
}

pub fn convert_u16_buffer(samples: &[u16]) -> Vec<i16> {
    samples.iter().copied().map(u16_to_i16).collect()
}

#[cfg(test)]
mod tests {
    use super::{convert_f32_buffer, convert_u16_buffer, f32_to_i16, u16_to_i16};

    #[test]
    fn converts_f32_sample_to_i16() {
        assert_eq!(f32_to_i16(0.0), 0);
        assert_eq!(f32_to_i16(1.0), i16::MAX);
        assert_eq!(f32_to_i16(-1.0), i16::MIN + 1);
        assert_eq!(f32_to_i16(2.0), i16::MAX);
        assert_eq!(f32_to_i16(-2.0), i16::MIN + 1);
    }

    #[test]
    fn converts_u16_sample_to_i16() {
        assert_eq!(u16_to_i16(0), i16::MIN);
        assert_eq!(u16_to_i16(32_768), 0);
        assert_eq!(u16_to_i16(u16::MAX), i16::MAX);
    }

    #[test]
    fn converts_f32_buffer_to_i16() {
        let converted = convert_f32_buffer(&[-1.0, -0.5, 0.0, 0.5, 1.0]);
        assert_eq!(converted.len(), 5);
        assert_eq!(converted[0], i16::MIN + 1);
        assert_eq!(converted[2], 0);
        assert_eq!(converted[4], i16::MAX);
    }

    #[test]
    fn converts_u16_buffer_to_i16() {
        let converted = convert_u16_buffer(&[0, 32_768, u16::MAX]);
        assert_eq!(converted, vec![i16::MIN, 0, i16::MAX]);
    }
}
