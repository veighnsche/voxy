use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::error::SttConfigError;

pub const VOXY_OPENAI_API_KEY_ENV: &str = "VOXY_OPENAI_API_KEY";
pub const VOXY_OPENAI_API_KEY_FILE_ENV: &str = "VOXY_OPENAI_API_KEY_FILE";
pub const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";
const DOTENV_FILES: [&str; 2] = [".env", ".env.local"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeySource {
    VoxyEnv,
    VoxyFile(PathBuf),
    OpenAiEnv,
    DotenvVoxyEnv(PathBuf),
    DotenvVoxyFile {
        dotenv_path: PathBuf,
        key_path: PathBuf,
    },
    DotenvOpenAiEnv(PathBuf),
}

impl ApiKeySource {
    pub fn description(&self) -> String {
        match self {
            Self::VoxyEnv => VOXY_OPENAI_API_KEY_ENV.to_owned(),
            Self::VoxyFile(path) => {
                format!("{} ({})", VOXY_OPENAI_API_KEY_FILE_ENV, path.display())
            }
            Self::OpenAiEnv => OPENAI_API_KEY_ENV.to_owned(),
            Self::DotenvVoxyEnv(path) => {
                format!("{} in {}", VOXY_OPENAI_API_KEY_ENV, path.display())
            }
            Self::DotenvVoxyFile {
                dotenv_path,
                key_path,
            } => format!(
                "{} in {} -> {}",
                VOXY_OPENAI_API_KEY_FILE_ENV,
                dotenv_path.display(),
                key_path.display()
            ),
            Self::DotenvOpenAiEnv(path) => {
                format!("{} in {}", OPENAI_API_KEY_ENV, path.display())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiKeyConfig {
    pub api_key: String,
    pub source: ApiKeySource,
}

pub fn load_api_key() -> Result<ApiKeyConfig, SttConfigError> {
    if let Some(config) = load_api_key_from_env()? {
        return Ok(config);
    }

    if let Some(config) = load_api_key_from_dotenv()? {
        return Ok(config);
    }

    Err(SttConfigError::MissingApiKey)
}

fn load_api_key_from_env() -> Result<Option<ApiKeyConfig>, SttConfigError> {
    if let Some(key) = non_empty_env(VOXY_OPENAI_API_KEY_ENV) {
        return Ok(Some(ApiKeyConfig {
            api_key: key,
            source: ApiKeySource::VoxyEnv,
        }));
    }

    if let Some(path) = non_empty_env(VOXY_OPENAI_API_KEY_FILE_ENV) {
        let path = PathBuf::from(path);
        let key = read_key_from_file(&path)?;
        return Ok(Some(ApiKeyConfig {
            api_key: key,
            source: ApiKeySource::VoxyFile(path),
        }));
    }

    if let Some(key) = non_empty_env(OPENAI_API_KEY_ENV) {
        return Ok(Some(ApiKeyConfig {
            api_key: key,
            source: ApiKeySource::OpenAiEnv,
        }));
    }

    Ok(None)
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

fn load_api_key_from_dotenv() -> Result<Option<ApiKeyConfig>, SttConfigError> {
    let cwd = match env::current_dir() {
        Ok(path) => path,
        Err(_) => return Ok(None),
    };

    load_api_key_from_dotenv_dir(&cwd)
}

fn load_api_key_from_dotenv_dir(dir: &Path) -> Result<Option<ApiKeyConfig>, SttConfigError> {
    let mut voxy_env = None;
    let mut voxy_file = None;
    let mut openai_env = None;

    for name in DOTENV_FILES {
        let path = dir.join(name);
        if !path.exists() {
            continue;
        }

        let entries = parse_dotenv_file(&path)?;
        if let Some(value) = entries.get(VOXY_OPENAI_API_KEY_ENV) {
            voxy_env = Some((value.clone(), path.clone()));
        }
        if let Some(value) = entries.get(VOXY_OPENAI_API_KEY_FILE_ENV) {
            voxy_file = Some((value.clone(), path.clone()));
        }
        if let Some(value) = entries.get(OPENAI_API_KEY_ENV) {
            openai_env = Some((value.clone(), path.clone()));
        }
    }

    if let Some((api_key, dotenv_path)) = voxy_env {
        return Ok(Some(ApiKeyConfig {
            api_key,
            source: ApiKeySource::DotenvVoxyEnv(dotenv_path),
        }));
    }

    if let Some((file_value, dotenv_path)) = voxy_file {
        let key_path = resolve_relative_path(&dotenv_path, &file_value);
        let api_key = read_key_from_file(&key_path)?;
        return Ok(Some(ApiKeyConfig {
            api_key,
            source: ApiKeySource::DotenvVoxyFile {
                dotenv_path,
                key_path,
            },
        }));
    }

    if let Some((api_key, dotenv_path)) = openai_env {
        return Ok(Some(ApiKeyConfig {
            api_key,
            source: ApiKeySource::DotenvOpenAiEnv(dotenv_path),
        }));
    }

    Ok(None)
}

fn parse_dotenv_file(
    path: &Path,
) -> Result<std::collections::HashMap<String, String>, SttConfigError> {
    let raw = fs::read_to_string(path).map_err(|source| SttConfigError::DotenvRead {
        path: path.to_path_buf(),
        source,
    })?;

    let mut values = std::collections::HashMap::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((key, value_raw)) = trimmed.split_once('=') else {
            continue;
        };

        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        let value = parse_env_value(value_raw);
        if value.is_empty() {
            continue;
        }

        values.insert(key.to_owned(), value);
    }

    Ok(values)
}

fn parse_env_value(value_raw: &str) -> String {
    let trimmed = value_raw.trim();
    if trimmed.len() >= 2 {
        let first = trimmed.as_bytes()[0];
        let last = *trimmed.as_bytes().last().unwrap_or(&first);
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return trimmed[1..trimmed.len() - 1].trim().to_owned();
        }
    }

    trimmed
        .split('#')
        .next()
        .map(str::trim)
        .unwrap_or_default()
        .to_owned()
}

fn resolve_relative_path(dotenv_path: &Path, file_value: &str) -> PathBuf {
    let candidate = PathBuf::from(file_value);
    if candidate.is_absolute() {
        return candidate;
    }

    dotenv_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(candidate)
}
