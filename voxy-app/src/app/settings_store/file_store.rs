use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

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

    let temp_path = temp_settings_path(&path);
    write_temp_file(&temp_path, format!("{json}\n").as_bytes())?;

    fs::rename(&temp_path, &path).map_err(|error| {
        format!(
            "failed to atomically replace settings file '{}' from '{}': {error}",
            path.display(),
            temp_path.display()
        )
    })?;

    sync_parent_dir(&path)
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

fn write_temp_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| {
            format!(
                "failed to open temp settings file '{}': {error}",
                path.display()
            )
        })?;

    file.write_all(contents).map_err(|error| {
        format!(
            "failed to write temp settings file '{}': {error}",
            path.display()
        )
    })?;
    file.sync_all().map_err(|error| {
        format!(
            "failed to sync temp settings file '{}': {error}",
            path.display()
        )
    })?;
    Ok(())
}

fn sync_parent_dir(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid settings path '{}'", path.display()))?;
    let dir = OpenOptions::new()
        .read(true)
        .open(parent)
        .map_err(|error| {
            format!(
                "failed to open settings directory '{}': {error}",
                parent.display()
            )
        })?;
    dir.sync_all().map_err(|error| {
        format!(
            "failed to sync settings directory '{}': {error}",
            parent.display()
        )
    })
}

fn temp_settings_path(path: &Path) -> PathBuf {
    let mut temp_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| "settings.json".into());
    temp_name.push(".tmp");
    path.with_file_name(temp_name)
}
