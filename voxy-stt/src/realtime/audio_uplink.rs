use base64::{engine::general_purpose::STANDARD, Engine as _};
use voxy_audio::PcmFrame;

const TARGET_SAMPLE_RATE_HZ: u32 = 24_000;
const TARGET_CHANNELS: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioAppendChunk {
    pub base64_pcm16: String,
}

pub fn encode_frame_to_base64(frame: &PcmFrame) -> Option<AudioAppendChunk> {
    if frame.is_empty() {
        return None;
    }

    let mono = downmix_to_mono(&frame.samples_i16, frame.channels);
    let resampled = resample_linear_i16(&mono, frame.sample_rate_hz, TARGET_SAMPLE_RATE_HZ);

    if resampled.is_empty() {
        return None;
    }

    let mut bytes = Vec::with_capacity(resampled.len() * 2);
    for sample in resampled {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }

    Some(AudioAppendChunk {
        base64_pcm16: STANDARD.encode(bytes),
    })
}

pub fn target_sample_rate_hz() -> u32 {
    TARGET_SAMPLE_RATE_HZ
}

pub fn target_channels() -> u16 {
    TARGET_CHANNELS
}

fn downmix_to_mono(samples: &[i16], channels: u16) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }

    let channels = channels as usize;
    samples
        .chunks(channels)
        .map(|chunk| {
            let sum = chunk.iter().map(|sample| *sample as i32).sum::<i32>();
            (sum / chunk.len() as i32) as i16
        })
        .collect()
}

fn resample_linear_i16(input: &[i16], in_rate_hz: u32, out_rate_hz: u32) -> Vec<i16> {
    if input.is_empty() {
        return Vec::new();
    }

    if in_rate_hz == out_rate_hz {
        return input.to_vec();
    }

    let out_len = ((input.len() as f64) * out_rate_hz as f64 / in_rate_hz as f64).round() as usize;
    if out_len == 0 {
        return Vec::new();
    }

    let step = in_rate_hz as f64 / out_rate_hz as f64;
    let mut output = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_pos = i as f64 * step;
        let left = src_pos.floor() as usize;
        let right = (left + 1).min(input.len() - 1);
        let frac = src_pos - left as f64;

        let left_sample = input[left] as f64;
        let right_sample = input[right] as f64;
        let sample = left_sample + (right_sample - left_sample) * frac;
        output.push(sample.round().clamp(i16::MIN as f64, i16::MAX as f64) as i16);
    }

    output
}

#[cfg(test)]
mod tests {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use voxy_audio::PcmFrame;

    use super::{encode_frame_to_base64, target_channels, target_sample_rate_hz};

    #[test]
    fn encodes_non_empty_pcm_frame() {
        let frame = PcmFrame::new(24_000, 1, vec![100, -100, 200, -200]);
        let chunk = encode_frame_to_base64(&frame).expect("chunk should encode");
        let bytes = STANDARD
            .decode(chunk.base64_pcm16)
            .expect("base64 should decode");
        assert_eq!(bytes.len(), 8);
    }

    #[test]
    fn downmixes_stereo_and_resamples_to_target_pcm() {
        // stereo interleaved: (1000, -1000), (2000, -2000), ...
        let frame = PcmFrame::new(
            16_000,
            2,
            vec![1000, -1000, 2000, -2000, 3000, -3000, 4000, -4000],
        );
        let chunk = encode_frame_to_base64(&frame).expect("chunk should encode");
        let bytes = STANDARD
            .decode(chunk.base64_pcm16)
            .expect("base64 should decode");
        // 4 mono samples at 16k -> ~6 samples at 24k, each 2 bytes
        assert_eq!(bytes.len(), 12);
        assert_eq!(target_sample_rate_hz(), 24_000);
        assert_eq!(target_channels(), 1);
    }
}
