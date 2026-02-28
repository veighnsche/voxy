use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::error::SttConfigError;

pub const VOXY_OPENAI_API_KEY_ENV: &str = "VOXY_OPENAI_API_KEY";
pub const VOXY_OPENAI_API_KEY_FILE_ENV: &str = "VOXY_OPENAI_API_KEY_FILE";
pub const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeySource {
    VoxyEnv,
    VoxyFile(PathBuf),
    OpenAiEnv,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiKeyConfig {
    pub api_key: String,
    pub source: ApiKeySource,
}

pub fn load_api_key() -> Result<ApiKeyConfig, SttConfigError> {
    if let Some(key) = non_empty_env(VOXY_OPENAI_API_KEY_ENV) {
        return Ok(ApiKeyConfig {
            api_key: key,
            source: ApiKeySource::VoxyEnv,
        });
    }

    if let Some(path) = non_empty_env(VOXY_OPENAI_API_KEY_FILE_ENV) {
        let path = PathBuf::from(path);
        let key = read_key_from_file(&path)?;
        return Ok(ApiKeyConfig {
            api_key: key,
            source: ApiKeySource::VoxyFile(path),
        });
    }

    if let Some(key) = non_empty_env(OPENAI_API_KEY_ENV) {
        return Ok(ApiKeyConfig {
            api_key: key,
            source: ApiKeySource::OpenAiEnv,
        });
    }

    Err(SttConfigError::MissingApiKey)
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn read_key_from_file(path: &Path) -> Result<String, SttConfigError> {
    let raw = fs::read_to_string(path).map_err(|source| SttConfigError::ApiKeyFileRead {
        path: path.to_path_buf(),
        source,
    })?;

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SttConfigError::ApiKeyFileEmpty {
            path: path.to_path_buf(),
        });
    }

    Ok(trimmed.to_owned())
}
