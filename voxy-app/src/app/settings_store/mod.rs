use serde::{Deserialize, Serialize};
use voxy_core::{
    clamp_silence_auto_stop_seconds, clamp_silence_gate_threshold, clamp_vad_silence_duration_ms,
};

mod file_store;
mod paths;

#[derive(Debug, Serialize, Deserialize, Default)]
struct SettingsFile {
    silence_auto_stop_seconds: Option<u64>,
    silence_gate_threshold: Option<f32>,
    vad_silence_ms: Option<u32>,
}

pub fn load_silence_auto_stop_seconds() -> Result<Option<u64>, String> {
    Ok(file_store::load_settings_file()?
        .silence_auto_stop_seconds
        .map(clamp_silence_auto_stop_seconds))
}

pub fn load_silence_gate_threshold() -> Result<Option<f32>, String> {
    Ok(file_store::load_settings_file()?
        .silence_gate_threshold
        .map(clamp_silence_gate_threshold))
}

pub fn load_vad_silence_ms() -> Result<Option<u32>, String> {
    Ok(file_store::load_settings_file()?
        .vad_silence_ms
        .map(clamp_vad_silence_duration_ms))
}

pub fn save_recording_settings(
    silence_auto_stop_seconds: u64,
    silence_gate_threshold: f32,
    vad_silence_ms: u32,
) -> Result<(), String> {
    let mut payload = file_store::load_settings_file()?;
    payload.silence_auto_stop_seconds =
        Some(clamp_silence_auto_stop_seconds(silence_auto_stop_seconds));
    payload.silence_gate_threshold = Some(clamp_silence_gate_threshold(silence_gate_threshold));
    payload.vad_silence_ms = Some(clamp_vad_silence_duration_ms(vad_silence_ms));
    file_store::save_settings_file(&payload)
}

#[cfg(test)]
pub fn settings_file_path() -> Option<std::path::PathBuf> {
    paths::settings_file_path()
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
        save_recording_settings, settings_file_path,
    };

    const XDG_CONFIG_HOME_ENV: &str = "XDG_CONFIG_HOME";
    const HOME_ENV: &str = "HOME";

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

        save_recording_settings(37, 0.42, 1650).expect("recording settings should save");

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
    fn save_recording_settings_updates_all_fields_in_one_write_path() {
        let _env_lock = ENV_LOCK.lock().expect("env mutex should not be poisoned");
        let dir = test_dir("save-recording-settings");
        fs::create_dir_all(&dir).expect("test directory should be created");

        let _xdg_guard = EnvVarGuard::set(XDG_CONFIG_HOME_ENV, dir.to_string_lossy().as_ref());
        let _home_guard = EnvVarGuard::unset(HOME_ENV);

        save_recording_settings(25, 0.55, 1500).expect("recording settings should save");

        assert_eq!(
            load_silence_auto_stop_seconds().expect("silence timeout should load"),
            Some(25)
        );
        assert_eq!(
            load_silence_gate_threshold().expect("gate threshold should load"),
            Some(0.55)
        );
        assert_eq!(
            load_vad_silence_ms().expect("vad silence should load"),
            Some(1500)
        );

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
