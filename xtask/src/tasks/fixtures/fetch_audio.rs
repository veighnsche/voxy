use std::{fs, path::Path, process::Command};

use anyhow::{bail, Context, Result};
use clap::Args;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::workspace;

#[derive(Debug, Clone, Args)]
pub struct FetchAudioArgs {
    #[arg(long, default_value = "tests/fixtures/audio/manifest.json")]
    pub manifest: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AudioFixtureManifest {
    fixture_name: String,
    file_name: String,
    url: String,
    sha256: String,
}

pub fn run(args: FetchAudioArgs) -> Result<()> {
    let root = workspace::root();
    let manifest_path = root.join(&args.manifest);
    let manifest = read_manifest(&manifest_path)?;
    let fixture_path = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(&manifest.file_name);

    fs::create_dir_all(
        fixture_path
            .parent()
            .context("fixture path should have a parent directory")?,
    )
    .with_context(|| {
        format!(
            "failed to create fixture directory '{}'",
            fixture_path.display()
        )
    })?;

    download_to_path(&manifest.url, &fixture_path)?;
    verify_sha256(&fixture_path, &manifest.sha256)?;

    println!(
        "[xtask] fetched fixture '{}' to {}",
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

fn download_to_path(url: &str, path: &Path) -> Result<()> {
    let status = Command::new("curl")
        .arg("-L")
        .arg("--fail")
        .arg("--silent")
        .arg("--show-error")
        .arg("-o")
        .arg(path)
        .arg(url)
        .status()
        .context("failed to execute curl for fixture download")?;

    if !status.success() {
        bail!("fixture download failed with status {status}");
    }

    Ok(())
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
