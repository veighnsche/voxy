#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManagedModel {
    WhisperLargeV3Turbo,
}

impl ManagedModel {
    pub const ALL: [Self; 1] = [Self::WhisperLargeV3Turbo];

    pub fn id(self) -> &'static str {
        match self {
            Self::WhisperLargeV3Turbo => "openai/whisper-large-v3-turbo",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::WhisperLargeV3Turbo => "Whisper Large V3 Turbo",
        }
    }

    pub fn vendor(self) -> &'static str {
        match self {
            Self::WhisperLargeV3Turbo => "OpenAI",
        }
    }

    pub fn size_gb(self) -> f32 {
        match self {
            Self::WhisperLargeV3Turbo => 1.62,
        }
    }

    pub fn artifact_file_name(self) -> &'static str {
        match self {
            Self::WhisperLargeV3Turbo => "openai_whisper_large_v3_turbo_model.safetensors",
        }
    }

    pub fn download_url(self) -> &'static str {
        match self {
            Self::WhisperLargeV3Turbo => {
                "https://huggingface.co/openai/whisper-large-v3-turbo/resolve/main/model.safetensors"
            }
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "openai/whisper-large-v3-turbo" => Some(Self::WhisperLargeV3Turbo),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ManagedModel;

    #[test]
    fn all_models_roundtrip_from_id() {
        for model in ManagedModel::ALL {
            let roundtrip =
                ManagedModel::from_id(model.id()).expect("model id should resolve from catalog");
            assert_eq!(roundtrip, model);
        }
    }

    #[test]
    fn catalog_metadata_is_stable_for_whisper_large_v3_turbo() {
        let model = ManagedModel::WhisperLargeV3Turbo;
        assert_eq!(model.id(), "openai/whisper-large-v3-turbo");
        assert_eq!(model.title(), "Whisper Large V3 Turbo");
        assert_eq!(model.vendor(), "OpenAI");
        assert!((model.size_gb() - 1.62).abs() < 0.001);
        assert_eq!(
            model.artifact_file_name(),
            "openai_whisper_large_v3_turbo_model.safetensors"
        );
        assert_eq!(
            model.download_url(),
            "https://huggingface.co/openai/whisper-large-v3-turbo/resolve/main/model.safetensors"
        );
    }

    #[test]
    fn unknown_model_id_returns_none() {
        assert_eq!(ManagedModel::from_id("not-a-model"), None);
    }
}
