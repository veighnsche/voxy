use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const XDG_CONFIG_HOME_ENV: &str = "XDG_CONFIG_HOME";
const HOME_ENV: &str = "HOME";
const APP_CONFIG_DIR: &str = "voxy";
const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, Serialize, Deserialize, Default)]
struct SettingsFile {
    silence_auto_stop_seconds: Option<u64>,
}

pub fn load_silence_auto_stop_seconds() -> Result<Option<u64>, String> {
    let Some(path) = settings_file_path() else {
        return Ok(None);
    };

    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read settings file '{}': {error}", path.display()))?;
    let parsed: SettingsFile = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "failed to parse settings file '{}': {error}",
            path.display()
        )
    })?;

    Ok(parsed.silence_auto_stop_seconds)
}

pub fn save_silence_auto_stop_seconds(seconds: u64) -> Result<(), String> {
    let Some(path) = settings_file_path() else {
        return Err("no config directory available (missing XDG_CONFIG_HOME and HOME)".to_owned());
    };

    ensure_parent_dir(&path)?;

    let payload = SettingsFile {
        silence_auto_stop_seconds: Some(seconds),
    };
    let json = serde_json::to_string_pretty(&payload).map_err(|error| {
        format!(
            "failed to serialize settings file payload '{}': {error}",
            path.display()
        )
    })?;

    fs::write(&path, format!("{json}\n")).map_err(|error| {
        format!(
            "failed to write settings file '{}': {error}",
            path.display()
        )
    })
}

pub fn settings_file_path() -> Option<PathBuf> {
    xdg_config_home().map(|dir| dir.join(APP_CONFIG_DIR).join(SETTINGS_FILE_NAME))
}

fn xdg_config_home() -> Option<PathBuf> {
    if let Some(dir) = non_empty_env(XDG_CONFIG_HOME_ENV) {
        return Some(PathBuf::from(dir));
    }

    non_empty_env(HOME_ENV).map(|home| PathBuf::from(home).join(".config"))
}

fn non_empty_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .and_then(|value| if value.is_empty() { None } else { Some(value) })
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid settings path '{}'", path.display()))?;

    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create settings directory '{}': {error}",
            parent.display()
        )
    })
}
