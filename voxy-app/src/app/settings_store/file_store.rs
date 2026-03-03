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

    load_settings_file_from_path(&path)
}

fn load_settings_file_from_path(path: &Path) -> Result<SettingsFile, String> {
    if !path.exists() {
        return Ok(SettingsFile::default());
    }

    let raw = fs::read_to_string(path)
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

    save_settings_file_to_path_with_injector(&path, payload, |_| {})
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SaveInjectionPoint {
    AfterTempWrite,
    AfterRename,
}

fn save_settings_file_to_path_with_injector(
    path: &Path,
    payload: &SettingsFile,
    mut injector: impl FnMut(SaveInjectionPoint),
) -> Result<(), String> {
    ensure_parent_dir(path)?;

    let json = serde_json::to_string_pretty(payload).map_err(|error| {
        format!(
            "failed to serialize settings file payload '{}': {error}",
            path.display()
        )
    })?;

    let temp_path = temp_settings_path(path);
    write_temp_file(&temp_path, format!("{json}\n").as_bytes())?;
    injector(SaveInjectionPoint::AfterTempWrite);

    fs::rename(&temp_path, path).map_err(|error| {
        format!(
            "failed to atomically replace settings file '{}' from '{}': {error}",
            path.display(),
            temp_path.display()
        )
    })?;
    injector(SaveInjectionPoint::AfterRename);

    sync_parent_dir(path)
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        panic::{self, AssertUnwindSafe},
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        load_settings_file_from_path, save_settings_file_to_path_with_injector, SaveInjectionPoint,
    };
    use crate::app::settings_store::SettingsFile;

    fn test_settings_path(test_name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "voxy-settings-durability-{test_name}-{}-{stamp}",
                std::process::id()
            ))
            .join("voxy")
            .join("settings.json")
    }

    fn write_payload(path: &PathBuf, payload: &SettingsFile) {
        let parent = path.parent().expect("settings path should have parent");
        fs::create_dir_all(parent).expect("settings directory should be created");
        let json =
            serde_json::to_string_pretty(payload).expect("test payload serialization should work");
        fs::write(path, format!("{json}\n")).expect("test payload should be written");
    }

    #[test]
    fn crash_after_temp_write_preserves_previous_good_file() {
        let path = test_settings_path("after-temp");
        let old_payload = SettingsFile {
            silence_auto_stop_seconds: Some(11),
            silence_gate_threshold: Some(0.33),
            vad_silence_ms: Some(1400),
        };
        let new_payload = SettingsFile {
            silence_auto_stop_seconds: Some(22),
            silence_gate_threshold: Some(0.66),
            vad_silence_ms: Some(1800),
        };
        write_payload(&path, &old_payload);

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _ = save_settings_file_to_path_with_injector(&path, &new_payload, |point| {
                if matches!(point, SaveInjectionPoint::AfterTempWrite) {
                    panic!("simulated process crash after temp-file fsync");
                }
            });
        }));
        assert!(result.is_err(), "expected simulated crash to panic");

        let loaded =
            load_settings_file_from_path(&path).expect("existing settings file should still load");
        assert_eq!(loaded, old_payload);

        let _ = fs::remove_dir_all(
            path.parent()
                .and_then(|p| p.parent())
                .unwrap_or_else(|| path.parent().expect("parent should exist")),
        );
    }

    #[test]
    fn crash_after_rename_keeps_new_good_file() {
        let path = test_settings_path("after-rename");
        let old_payload = SettingsFile {
            silence_auto_stop_seconds: Some(7),
            silence_gate_threshold: Some(0.22),
            vad_silence_ms: Some(1200),
        };
        let new_payload = SettingsFile {
            silence_auto_stop_seconds: Some(19),
            silence_gate_threshold: Some(0.71),
            vad_silence_ms: Some(2000),
        };
        write_payload(&path, &old_payload);

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _ = save_settings_file_to_path_with_injector(&path, &new_payload, |point| {
                if matches!(point, SaveInjectionPoint::AfterRename) {
                    panic!("simulated process crash after atomic rename");
                }
            });
        }));
        assert!(result.is_err(), "expected simulated crash to panic");

        let loaded =
            load_settings_file_from_path(&path).expect("existing settings file should still load");
        assert_eq!(loaded, new_payload);

        let _ = fs::remove_dir_all(
            path.parent()
                .and_then(|p| p.parent())
                .unwrap_or_else(|| path.parent().expect("parent should exist")),
        );
    }
}
