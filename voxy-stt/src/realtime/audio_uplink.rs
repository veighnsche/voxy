#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioAppendChunk {
    pub base64_pcm16: String,
}

pub fn encode_pcm16_to_base64(_samples: &[i16]) -> AudioAppendChunk {
    AudioAppendChunk {
        base64_pcm16: String::new(),
    }
}
