use std::{env, path::PathBuf};

pub(super) const XDG_CONFIG_HOME_ENV: &str = "XDG_CONFIG_HOME";
pub(super) const HOME_ENV: &str = "HOME";
const APP_CONFIG_DIR: &str = "voxy";
const SETTINGS_FILE_NAME: &str = "settings.json";

pub(super) fn settings_file_path() -> Option<PathBuf> {
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
