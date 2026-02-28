use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use clap::Args;
use voxy_stt::config::{
    self, ApiKeySource, OPENAI_API_KEY_ENV, VOXY_OPENAI_API_KEY_ENV, VOXY_OPENAI_API_KEY_FILE_ENV,
};
use voxy_stt::error::SttConfigError;

use crate::tasks::{fixtures, gui::smoke};
use crate::workspace;

#[derive(Debug, Clone, Args)]
pub struct SttE2eArgs {
    #[arg(long, default_value_t = false)]
    pub no_build: bool,
    #[arg(long)]
    pub fixture_id: Option<u8>,
    #[arg(long, default_value = "gpt-4o-mini-transcribe")]
    pub model: String,
}

pub fn run(args: SttE2eArgs) -> Result<()> {
    if !env_flag_enabled("VOXY_E2E_LIVE") {
        bail!("stt-e2e is opt-in. Set VOXY_E2E_LIVE=1 and ensure API key env is configured");
    }

    let root = workspace::root();
    let fixtures_dir = root.join("tests/fixtures/audio");
    fixtures::list_audio::ensure_required_fixtures_exist(&fixtures_dir)?;

    smoke::run(smoke::SmokeArgs {
        startup_ms: 2000,
        shutdown_timeout_ms: 5000,
        no_build: args.no_build,
    })?;

    let api_key = load_api_key(&root)?;
    let fixture_ids = match args.fixture_id {
        Some(id) => vec![id],
        None => fixtures::list_audio::fixture_ids().to_vec(),
    };

    for fixture_id in fixture_ids {
        run_fixture_transcription(&fixtures_dir, fixture_id, &api_key, &args.model)?;
    }

    Ok(())
}

fn run_fixture_transcription(
    fixtures_dir: &Path,
    fixture_id: u8,
    api_key: &str,
    model: &str,
) -> Result<()> {
    let audio_path = fixtures_dir.join(format!("test_{fixture_id}.mp3"));
    let expected_path = fixtures_dir.join(format!("test_{fixture_id}.txt"));
    let result_path = fixtures_dir.join(format!("test_{fixture_id}.result.txt"));

    if !audio_path.exists() {
        bail!(
            "fixture audio file '{}' does not exist",
            audio_path.display()
        );
    }
    if !expected_path.exists() {
        bail!(
            "fixture transcript file '{}' does not exist",
            expected_path.display()
        );
    }

    let transcript = transcribe_with_curl(&audio_path, api_key, model)?;
    let transcript_trimmed = transcript.trim();

    fs::write(&result_path, format!("{transcript_trimmed}\n")).with_context(|| {
        format!(
            "failed to write transcription output '{}'",
            result_path.display()
        )
    })?;

    let expected = fs::read_to_string(&expected_path).with_context(|| {
        format!(
            "failed to read expected transcript '{}'",
            expected_path.display()
        )
    })?;
    emit_match_hint(&expected, transcript_trimmed);

    println!(
        "[xtask] stt-e2e wrote live transcription for test_{} to {}",
        fixture_id,
        result_path.display()
    );

    Ok(())
}

fn env_flag_enabled(name: &str) -> bool {
    env::var(name)
        .ok()
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

fn load_api_key(root: &Path) -> Result<String> {
    match config::load_api_key() {
        Ok(cfg) => {
            println!(
                "[xtask] using API key from {}",
                describe_source(&cfg.source)
            );
            return Ok(cfg.api_key);
        }
        Err(SttConfigError::MissingApiKey) => {}
        Err(error) => return Err(error).context("failed to load API key via voxy-stt config"),
    }

    if let Some(dotenv) = load_api_key_from_dotenv(root)? {
        println!("[xtask] using API key from {}", dotenv.source);
        return Ok(dotenv.api_key);
    }

    bail!(
        "missing API key. Set one of: {}, {}, {}. For local dev, put one of these keys in .env.local or .env",
        VOXY_OPENAI_API_KEY_ENV,
        VOXY_OPENAI_API_KEY_FILE_ENV,
        OPENAI_API_KEY_ENV
    )
}

#[derive(Debug, Clone)]
struct DotenvApiKey {
    api_key: String,
    source: String,
}

fn load_api_key_from_dotenv(root: &Path) -> Result<Option<DotenvApiKey>> {
    // Precedence: .env.local overrides .env.
    let candidates = [root.join(".env"), root.join(".env.local")];
    let mut voxy_env = None;
    let mut voxy_file = None;
    let mut openai_env = None;

    for path in &candidates {
        if !path.exists() {
            continue;
        }
        let entries = parse_dotenv(path)?;
        if let Some(value) = entries.get(VOXY_OPENAI_API_KEY_ENV) {
            voxy_env = Some((value.clone(), path.clone()));
        }
        if let Some(value) = entries.get(VOXY_OPENAI_API_KEY_FILE_ENV) {
            voxy_file = Some((value.clone(), path.clone()));
        }
        if let Some(value) = entries.get(OPENAI_API_KEY_ENV) {
            openai_env = Some((value.clone(), path.clone()));
        }
    }

    if let Some((value, path)) = voxy_env {
        return Ok(Some(DotenvApiKey {
            api_key: value,
            source: format!("{} in {}", VOXY_OPENAI_API_KEY_ENV, path.display()),
        }));
    }

    if let Some((value, dotenv_path)) = voxy_file {
        let key_path = resolve_relative_path(&dotenv_path, &value);
        let raw = fs::read_to_string(&key_path).with_context(|| {
            format!(
                "failed to read API key file '{}' from {}",
                key_path.display(),
                VOXY_OPENAI_API_KEY_FILE_ENV
            )
        })?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            bail!("API key file '{}' is empty", key_path.display());
        }
        return Ok(Some(DotenvApiKey {
            api_key: trimmed.to_owned(),
            source: format!(
                "{} in {}",
                VOXY_OPENAI_API_KEY_FILE_ENV,
                dotenv_path.display()
            ),
        }));
    }

    if let Some((value, path)) = openai_env {
        return Ok(Some(DotenvApiKey {
            api_key: value,
            source: format!("{} in {}", OPENAI_API_KEY_ENV, path.display()),
        }));
    }

    Ok(None)
}

fn parse_dotenv(path: &Path) -> Result<std::collections::HashMap<String, String>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read dotenv file '{}'", path.display()))?;
    let mut values = std::collections::HashMap::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((key, value_raw)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        let value = parse_env_value(value_raw);
        if value.is_empty() {
            continue;
        }
        values.insert(key.to_owned(), value);
    }

    Ok(values)
}

fn parse_env_value(value_raw: &str) -> String {
    let trimmed = value_raw.trim();
    if trimmed.len() >= 2 {
        let first = trimmed.as_bytes()[0];
        let last = *trimmed.as_bytes().last().unwrap_or(&first);
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return trimmed[1..trimmed.len() - 1].trim().to_owned();
        }
    }

    trimmed
        .split('#')
        .next()
        .map(str::trim)
        .unwrap_or_default()
        .to_owned()
}

fn resolve_relative_path(dotenv_path: &Path, file_value: &str) -> PathBuf {
    let candidate = PathBuf::from(file_value);
    if candidate.is_absolute() {
        return candidate;
    }

    dotenv_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(candidate)
}

fn describe_source(source: &ApiKeySource) -> String {
    match source {
        ApiKeySource::VoxyEnv => VOXY_OPENAI_API_KEY_ENV.to_owned(),
        ApiKeySource::VoxyFile(path) => {
            format!("{} ({})", VOXY_OPENAI_API_KEY_FILE_ENV, path.display())
        }
        ApiKeySource::OpenAiEnv => OPENAI_API_KEY_ENV.to_owned(),
    }
}

fn transcribe_with_curl(audio_path: &Path, api_key: &str, model: &str) -> Result<String> {
    let output = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--fail")
        .arg("https://api.openai.com/v1/audio/transcriptions")
        .arg("-H")
        .arg(format!("Authorization: Bearer {api_key}"))
        .arg("-F")
        .arg(format!("file=@{}", audio_path.display()))
        .arg("-F")
        .arg(format!("model={model}"))
        .arg("-F")
        .arg("response_format=text")
        .output()
        .context("failed to execute curl for live transcription")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("live transcription request failed: {}", stderr.trim());
    }

    let stdout =
        String::from_utf8(output.stdout).context("transcription response was not valid UTF-8")?;

    Ok(stdout)
}

fn emit_match_hint(expected: &str, actual: &str) {
    let expected_norm = normalize(expected);
    if expected_norm.is_empty() {
        return;
    }

    let actual_norm = normalize(actual);
    if actual_norm.contains(&expected_norm) {
        println!("[xtask] transcript contains expected text");
    } else {
        println!("[xtask] warning: transcript does not fully match expected text");
    }
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
