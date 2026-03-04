use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::error::SttConfigError;

pub const VOXY_OPENAI_API_KEY_ENV: &str = "VOXY_OPENAI_API_KEY";
pub const VOXY_OPENAI_API_KEY_FILE_ENV: &str = "VOXY_OPENAI_API_KEY_FILE";
pub const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";
const DOTENV_ENABLED_ENV: &str = "VOXY_STT_DOTENV_ENABLED";
const DOTENV_DIR_ENV: &str = "VOXY_STT_DOTENV_DIR";
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
    pub fn redacted_description(&self) -> &'static str {
        match self {
            Self::VoxyEnv => VOXY_OPENAI_API_KEY_ENV,
            Self::VoxyFile(_) => VOXY_OPENAI_API_KEY_FILE_ENV,
            Self::OpenAiEnv => OPENAI_API_KEY_ENV,
            Self::DotenvVoxyEnv(_) => "dotenv:VOXY_OPENAI_API_KEY",
            Self::DotenvVoxyFile { .. } => "dotenv:VOXY_OPENAI_API_KEY_FILE",
            Self::DotenvOpenAiEnv(_) => "dotenv:OPENAI_API_KEY",
        }
    }

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
    if !dotenv_enabled() {
        return Ok(None);
    }

    if let Some(dotenv_dir) = non_empty_env(DOTENV_DIR_ENV).map(PathBuf::from) {
        return load_api_key_from_dotenv_dir(&dotenv_dir);
    }

    load_api_key_from_dotenv_dirs(default_dotenv_dirs())
}

fn default_dotenv_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    push_unique_path(&mut dirs, env::current_dir().ok());
    push_unique_path(&mut dirs, default_config_dir());
    dirs
}

fn push_unique_path(dirs: &mut Vec<PathBuf>, candidate: Option<PathBuf>) {
    let Some(candidate) = candidate else {
        return;
    };

    if !dirs.iter().any(|dir| dir == &candidate) {
        dirs.push(candidate);
    }
}

fn load_api_key_from_dotenv_dirs<I>(dirs: I) -> Result<Option<ApiKeyConfig>, SttConfigError>
where
    I: IntoIterator<Item = PathBuf>,
{
    for dotenv_dir in dirs {
        if let Some(config) = load_api_key_from_dotenv_dir(&dotenv_dir)? {
            return Ok(Some(config));
        }
    }

    Ok(None)
}

fn default_config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        return non_empty_env("APPDATA")
            .map(PathBuf::from)
            .map(|dir| dir.join("voxy"));
    }

    #[cfg(not(windows))]
    {
        if let Some(xdg) = non_empty_env("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg).join("voxy"));
        }

        non_empty_env("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".config").join("voxy"))
    }
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
        if !path.is_file() {
            continue;
        }

        let mut entries = parse_dotenv_file(&path)?;
        if let Some(value) = entries.remove(VOXY_OPENAI_API_KEY_ENV) {
            voxy_env = Some((value, path.clone()));
        }
        if let Some(value) = entries.remove(VOXY_OPENAI_API_KEY_FILE_ENV) {
            voxy_file = Some((value, path.clone()));
        }
        if let Some(value) = entries.remove(OPENAI_API_KEY_ENV) {
            openai_env = Some((value, path.clone()));
        }
    }

    if let Some((api_key, dotenv_path)) = voxy_env {
        return Ok(Some(ApiKeyConfig {
            api_key,
            source: ApiKeySource::DotenvVoxyEnv(dotenv_path),
        }));
    }

    if let Some((file_value, dotenv_path)) = voxy_file {
        let key_path = resolve_relative_path(&dotenv_path, &file_value)?;
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

fn resolve_relative_path(dotenv_path: &Path, file_value: &str) -> Result<PathBuf, SttConfigError> {
    let candidate = PathBuf::from(file_value);
    if candidate.is_absolute() {
        return Err(SttConfigError::DotenvFilePathOutsideBase {
            dotenv_path: dotenv_path.to_path_buf(),
            file_value: file_value.to_owned(),
        });
    }

    let Some(normalized_relative) = normalize_relative_path(&candidate) else {
        return Err(SttConfigError::DotenvFilePathOutsideBase {
            dotenv_path: dotenv_path.to_path_buf(),
            file_value: file_value.to_owned(),
        });
    };

    Ok(dotenv_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(normalized_relative))
}

fn normalize_relative_path(path: &Path) -> Option<PathBuf> {
    use std::path::Component;

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(value) => normalized.push(value),
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}

fn dotenv_enabled() -> bool {
    parse_bool_env(non_empty_env(DOTENV_ENABLED_ENV).as_deref()).unwrap_or(true)
}

fn parse_bool_env(value: Option<&str>) -> Option<bool> {
    value.map(str::trim).and_then(|value| {
        let value = value.to_ascii_lowercase();
        match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::error::SttConfigError;

    use super::{
        load_api_key_from_dotenv_dir, load_api_key_from_dotenv_dirs, normalize_relative_path,
        parse_bool_env, push_unique_path, ApiKeySource,
    };

    fn test_dir(test_name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "voxy-stt-config-{test_name}-{}-{stamp}",
            std::process::id()
        ))
    }

    #[test]
    fn parse_bool_env_accepts_expected_values() {
        assert_eq!(parse_bool_env(Some("true")), Some(true));
        assert_eq!(parse_bool_env(Some("YES")), Some(true));
        assert_eq!(parse_bool_env(Some("0")), Some(false));
        assert_eq!(parse_bool_env(Some("off")), Some(false));
        assert_eq!(parse_bool_env(Some("invalid")), None);
    }

    #[test]
    fn normalize_relative_path_rejects_escape_attempts() {
        assert_eq!(
            normalize_relative_path(Path::new("keys/prod.txt")),
            Some(PathBuf::from("keys/prod.txt"))
        );
        assert_eq!(
            normalize_relative_path(Path::new("./keys/../prod.txt")),
            Some(PathBuf::from("prod.txt"))
        );
        assert_eq!(normalize_relative_path(Path::new("../prod.txt")), None);
        assert_eq!(normalize_relative_path(Path::new("../../prod.txt")), None);
    }

    #[test]
    fn dotenv_key_file_path_must_stay_within_dotenv_dir() {
        let dir = test_dir("dotenv-key-path-bounds");
        fs::create_dir_all(&dir).expect("test directory should be created");
        fs::write(
            dir.join(".env"),
            "VOXY_OPENAI_API_KEY_FILE=../outside-key.txt\n",
        )
        .expect("dotenv should be written");

        let error = load_api_key_from_dotenv_dir(&dir).expect_err("path escape should fail");
        assert!(
            matches!(error, SttConfigError::DotenvFilePathOutsideBase { .. }),
            "expected path bounds error, got: {error}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dotenv_key_file_path_allows_relative_path_inside_dir() {
        let dir = test_dir("dotenv-key-path-inside");
        let key_dir = dir.join("keys");
        fs::create_dir_all(&key_dir).expect("test key directory should be created");
        fs::write(key_dir.join("api.txt"), "sk-test-key\n")
            .expect("api key file should be written");
        fs::write(dir.join(".env"), "VOXY_OPENAI_API_KEY_FILE=keys/api.txt\n")
            .expect("dotenv should be written");

        let config = load_api_key_from_dotenv_dir(&dir)
            .expect("dotenv load should not fail")
            .expect("api key config should be resolved");
        assert_eq!(config.api_key, "sk-test-key");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn push_unique_path_skips_none_and_duplicates() {
        let mut dirs = Vec::new();
        push_unique_path(&mut dirs, None);
        push_unique_path(&mut dirs, Some(PathBuf::from("/tmp/voxy-config")));
        push_unique_path(&mut dirs, Some(PathBuf::from("/tmp/voxy-config")));

        assert_eq!(dirs, vec![PathBuf::from("/tmp/voxy-config")]);
    }

    #[test]
    fn dotenv_fallback_uses_first_matching_directory() {
        let first = test_dir("dotenv-fallback-first");
        let second = test_dir("dotenv-fallback-second");

        fs::create_dir_all(&first).expect("first fallback directory should be created");
        fs::create_dir_all(&second).expect("second fallback directory should be created");
        fs::write(first.join(".env"), "VOXY_OPENAI_API_KEY=sk-first\n")
            .expect("first dotenv should be written");
        fs::write(second.join(".env"), "VOXY_OPENAI_API_KEY=sk-second\n")
            .expect("second dotenv should be written");

        let config = load_api_key_from_dotenv_dirs(vec![first.clone(), second.clone()])
            .expect("fallback lookup should not fail")
            .expect("fallback should resolve an api key");

        assert_eq!(config.api_key, "sk-first");
        assert_eq!(
            config.source,
            ApiKeySource::DotenvVoxyEnv(first.join(".env"))
        );

        let _ = fs::remove_dir_all(&first);
        let _ = fs::remove_dir_all(&second);
    }

    #[test]
    fn dotenv_fallback_tries_next_directory_when_first_has_no_dotenv() {
        let first = test_dir("dotenv-fallback-empty");
        let second = test_dir("dotenv-fallback-next");

        fs::create_dir_all(&first).expect("empty fallback directory should be created");
        fs::create_dir_all(&second).expect("next fallback directory should be created");
        fs::write(second.join(".env"), "VOXY_OPENAI_API_KEY=sk-next\n")
            .expect("next dotenv should be written");

        let config = load_api_key_from_dotenv_dirs(vec![first.clone(), second.clone()])
            .expect("fallback lookup should not fail")
            .expect("fallback should resolve an api key");

        assert_eq!(config.api_key, "sk-next");
        assert_eq!(
            config.source,
            ApiKeySource::DotenvVoxyEnv(second.join(".env"))
        );

        let _ = fs::remove_dir_all(&first);
        let _ = fs::remove_dir_all(&second);
    }

    #[test]
    fn dotenv_directory_named_env_is_ignored() {
        let dir = test_dir("dotenv-dir-entry");
        fs::create_dir_all(dir.join(".env")).expect("directory named .env should be created");

        let config = load_api_key_from_dotenv_dir(&dir).expect("dotenv dir load should not fail");
        assert!(config.is_none(), "directory named .env should be ignored");

        let _ = fs::remove_dir_all(&dir);
    }
}
