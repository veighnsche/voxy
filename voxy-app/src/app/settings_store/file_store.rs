use std::{fs, path::Path};

use super::{paths, SettingsFile};

pub(super) fn load_settings_file() -> Result<SettingsFile, String> {
    let Some(path) = paths::settings_file_path() else {
        return Ok(SettingsFile::default());
    };

    if !path.exists() {
        return Ok(SettingsFile::default());
    }

    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read settings file '{}': {error}", path.display()))?;
    let parsed: SettingsFile = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "failed to parse settings file '{}': {error}",
            path.display()
        )
    })?;

    Ok(parsed)
}

pub(super) fn save_settings_file(payload: &SettingsFile) -> Result<(), String> {
    let Some(path) = paths::settings_file_path() else {
        return Err("no config directory available (missing XDG_CONFIG_HOME and HOME)".to_owned());
    };

    ensure_parent_dir(&path)?;

    let json = serde_json::to_string_pretty(payload).map_err(|error| {
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
