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
    silence_gate_threshold: Option<f32>,
    vad_silence_ms: Option<u32>,
}

pub fn load_silence_auto_stop_seconds() -> Result<Option<u64>, String> {
    Ok(load_settings_file()?.silence_auto_stop_seconds)
}

pub fn load_silence_gate_threshold() -> Result<Option<f32>, String> {
    Ok(load_settings_file()?
        .silence_gate_threshold
        .map(|value| value.clamp(0.0, 1.0)))
}

pub fn load_vad_silence_ms() -> Result<Option<u32>, String> {
    Ok(load_settings_file()?
        .vad_silence_ms
        .map(|value| value.clamp(100, 5_000)))
}

pub fn save_silence_auto_stop_seconds(seconds: u64) -> Result<(), String> {
    let mut payload = load_settings_file()?;
    payload.silence_auto_stop_seconds = Some(seconds);
    save_settings_file(&payload)
}

pub fn save_silence_gate_threshold(threshold: f32) -> Result<(), String> {
    let mut payload = load_settings_file()?;
    payload.silence_gate_threshold = Some(threshold.clamp(0.0, 1.0));
    save_settings_file(&payload)
}

pub fn save_vad_silence_ms(vad_silence_ms: u32) -> Result<(), String> {
    let mut payload = load_settings_file()?;
    payload.vad_silence_ms = Some(vad_silence_ms.clamp(100, 5_000));
    save_settings_file(&payload)
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

fn load_settings_file() -> Result<SettingsFile, String> {
    let Some(path) = settings_file_path() else {
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

fn save_settings_file(payload: &SettingsFile) -> Result<(), String> {
    let Some(path) = settings_file_path() else {
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::Mutex,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        load_silence_auto_stop_seconds, load_silence_gate_threshold, load_vad_silence_ms,
        save_silence_auto_stop_seconds, save_silence_gate_threshold, save_vad_silence_ms,
        settings_file_path, HOME_ENV, XDG_CONFIG_HOME_ENV,
    };

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvVarGuard {
        key: String,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe { std::env::set_var(key, value) };
            Self {
                key: key.to_owned(),
                previous,
            }
        }

        fn unset(key: &str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe { std::env::remove_var(key) };
            Self {
                key: key.to_owned(),
                previous,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(&self.key, value) },
                None => unsafe { std::env::remove_var(&self.key) },
            }
        }
    }

    fn test_dir(test_name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "voxy-settings-store-{test_name}-{}-{stamp}",
            std::process::id()
        ))
    }

    #[test]
    fn roundtrips_persisted_settings_in_xdg_config_home() {
        let _env_lock = ENV_LOCK.lock().expect("env mutex should not be poisoned");
        let dir = test_dir("roundtrip");
        fs::create_dir_all(&dir).expect("test directory should be created");

        let _xdg_guard = EnvVarGuard::set(XDG_CONFIG_HOME_ENV, dir.to_string_lossy().as_ref());
        let _home_guard = EnvVarGuard::unset(HOME_ENV);

        save_silence_auto_stop_seconds(37).expect("silence timeout should save");
        save_silence_gate_threshold(0.42).expect("gate threshold should save");
        save_vad_silence_ms(1650).expect("vad silence should save");

        assert_eq!(
            load_silence_auto_stop_seconds().expect("silence timeout should load"),
            Some(37)
        );
        let gate = load_silence_gate_threshold()
            .expect("gate threshold should load")
            .expect("gate threshold should exist");
        assert!((gate - 0.42).abs() <= f32::EPSILON);
        assert_eq!(
            load_vad_silence_ms().expect("vad silence should load"),
            Some(1650)
        );

        let expected_path = dir.join("voxy").join("settings.json");
        assert_eq!(settings_file_path(), Some(expected_path));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clamps_out_of_range_values_when_loading() {
        let _env_lock = ENV_LOCK.lock().expect("env mutex should not be poisoned");
        let dir = test_dir("clamp");
        let config_dir = dir.join("voxy");
        fs::create_dir_all(&config_dir).expect("test config dir should be created");

        let _xdg_guard = EnvVarGuard::set(XDG_CONFIG_HOME_ENV, dir.to_string_lossy().as_ref());
        let _home_guard = EnvVarGuard::unset(HOME_ENV);

        let payload = r#"{
  "silence_auto_stop_seconds": 11,
  "silence_gate_threshold": 9.9,
  "vad_silence_ms": 99999
}
"#;
        fs::write(config_dir.join("settings.json"), payload)
            .expect("test payload should be written");

        assert_eq!(
            load_silence_auto_stop_seconds().expect("silence timeout should load"),
            Some(11)
        );
        assert_eq!(
            load_silence_gate_threshold().expect("gate threshold should load"),
            Some(1.0)
        );
        assert_eq!(
            load_vad_silence_ms().expect("vad silence should load"),
            Some(5_000)
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn returns_error_for_invalid_json_payload() {
        let _env_lock = ENV_LOCK.lock().expect("env mutex should not be poisoned");
        let dir = test_dir("invalid-json");
        let config_dir = dir.join("voxy");
        fs::create_dir_all(&config_dir).expect("test config dir should be created");

        let _xdg_guard = EnvVarGuard::set(XDG_CONFIG_HOME_ENV, dir.to_string_lossy().as_ref());
        let _home_guard = EnvVarGuard::unset(HOME_ENV);

        fs::write(config_dir.join("settings.json"), "{ this is invalid json")
            .expect("invalid payload should be written");

        let error =
            load_silence_auto_stop_seconds().expect_err("invalid payload should return error");
        assert!(
            error.contains("failed to parse settings file"),
            "expected parse error, got: {error}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn returns_none_when_settings_file_is_missing() {
        let _env_lock = ENV_LOCK.lock().expect("env mutex should not be poisoned");
        let dir = test_dir("missing-file");
        fs::create_dir_all(&dir).expect("test directory should be created");

        let _xdg_guard = EnvVarGuard::set(XDG_CONFIG_HOME_ENV, dir.to_string_lossy().as_ref());
        let _home_guard = EnvVarGuard::unset(HOME_ENV);

        assert_eq!(
            load_silence_auto_stop_seconds().expect("missing file should return default"),
            None
        );
        assert_eq!(
            load_silence_gate_threshold().expect("missing file should return default"),
            None
        );
        assert_eq!(
            load_vad_silence_ms().expect("missing file should return default"),
            None
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn loads_partial_settings_payload_without_overwriting_missing_fields() {
        let _env_lock = ENV_LOCK.lock().expect("env mutex should not be poisoned");
        let dir = test_dir("partial-payload");
        let config_dir = dir.join("voxy");
        fs::create_dir_all(&config_dir).expect("test config dir should be created");

        let _xdg_guard = EnvVarGuard::set(XDG_CONFIG_HOME_ENV, dir.to_string_lossy().as_ref());
        let _home_guard = EnvVarGuard::unset(HOME_ENV);

        let payload = r#"{
  "silence_auto_stop_seconds": 9
}
"#;
        fs::write(config_dir.join("settings.json"), payload)
            .expect("partial payload should be written");

        assert_eq!(
            load_silence_auto_stop_seconds().expect("silence timeout should load"),
            Some(9)
        );
        assert_eq!(
            load_silence_gate_threshold().expect("gate threshold should load"),
            None
        );
        assert_eq!(
            load_vad_silence_ms().expect("vad silence should load"),
            None
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
