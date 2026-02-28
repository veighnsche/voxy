use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use clap::Args;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::workspace;

#[derive(Debug, Clone, Args)]
pub struct VerifyAudioArgs {
    #[arg(long, default_value = "tests/fixtures/audio/manifest.json")]
    pub manifest: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AudioFixtureManifest {
    fixture_name: String,
    file_name: String,
    sha256: String,
    expected_substring: String,
}

pub fn run(args: VerifyAudioArgs) -> Result<()> {
    let root = workspace::root();
    let manifest_path = root.join(&args.manifest);
    let manifest = read_manifest(&manifest_path)?;

    let fixture_path = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(&manifest.file_name);

    if !fixture_path.exists() {
        bail!(
            "fixture file '{}' is missing. Run: cargo run -p xtask -- fixtures fetch-audio",
            fixture_path.display()
        );
    }

    verify_sha256(&fixture_path, &manifest.sha256)?;

    let expected_path = fixture_path.with_extension("expected.txt");
    if !expected_path.exists() {
        bail!(
            "expected transcript file '{}' is missing",
            expected_path.display()
        );
    }

    let expected = fs::read_to_string(&expected_path)
        .with_context(|| format!("failed to read '{}'", expected_path.display()))?;
    if !expected
        .to_ascii_lowercase()
        .contains(&manifest.expected_substring.to_ascii_lowercase())
    {
        bail!(
            "expected transcript in '{}' does not contain required substring '{}'",
            expected_path.display(),
            manifest.expected_substring
        );
    }

    println!(
        "[xtask] verified fixture '{}' ({})",
        manifest.fixture_name,
        fixture_path.display()
    );

    Ok(())
}

fn read_manifest(path: &Path) -> Result<AudioFixtureManifest> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read fixture manifest '{}'", path.display()))?;
    let manifest = serde_json::from_str::<AudioFixtureManifest>(&raw)
        .with_context(|| format!("invalid fixture manifest JSON '{}'", path.display()))?;
    Ok(manifest)
}

fn verify_sha256(path: &Path, expected_sha256: &str) -> Result<()> {
    let actual = compute_sha256(path)?;
    if actual != expected_sha256.to_ascii_lowercase() {
        bail!(
            "fixture checksum mismatch for '{}': expected {}, got {}",
            path.display(),
            expected_sha256,
            actual
        );
    }

    Ok(())
}

fn compute_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read fixture bytes from '{}'", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}
