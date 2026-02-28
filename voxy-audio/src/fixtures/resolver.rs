use std::path::{Path, PathBuf};

use crate::AudioError;

pub fn default_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root should be parent of voxy-audio")
        .join("tests")
        .join("fixtures")
        .join("audio")
}

pub fn resolve_fixture_mp3(root: &Path, fixture_name: &str) -> Result<PathBuf, AudioError> {
    validate_fixture_name(fixture_name)?;

    let path = root.join(format!("{fixture_name}.mp3"));
    if !path.is_file() {
        return Err(AudioError::FixtureNotFound(path));
    }

    Ok(path)
}

fn validate_fixture_name(fixture_name: &str) -> Result<(), AudioError> {
    if !fixture_name.starts_with("test_") {
        return Err(AudioError::InvalidFixtureName(fixture_name.to_owned()));
    }

    let Some(index) = fixture_name.strip_prefix("test_") else {
        return Err(AudioError::InvalidFixtureName(fixture_name.to_owned()));
    };

    if index.is_empty() || !index.chars().all(|c| c.is_ascii_digit()) {
        return Err(AudioError::InvalidFixtureName(fixture_name.to_owned()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_fixture_name;

    #[test]
    fn valid_fixture_name_passes() {
        assert!(validate_fixture_name("test_3").is_ok());
    }

    #[test]
    fn invalid_fixture_name_fails() {
        assert!(validate_fixture_name("voice_3").is_err());
        assert!(validate_fixture_name("test_x").is_err());
    }
}
