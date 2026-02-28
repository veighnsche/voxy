use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use clap::Args;

use crate::workspace;

const FIXTURE_IDS: [u8; 5] = [1, 2, 3, 4, 5];

#[derive(Debug, Clone, Args)]
pub struct ListAudioArgs {
    #[arg(long, default_value = "tests/fixtures/audio")]
    pub fixtures_dir: String,
}

#[derive(Debug, Clone)]
pub struct FixturePair {
    pub id: u8,
    pub audio_path: String,
    pub transcript_path: String,
}

pub fn run(args: ListAudioArgs) -> Result<()> {
    let root = workspace::root();
    let fixtures_dir = root.join(&args.fixtures_dir);

    let pairs = required_fixture_pairs(&fixtures_dir);
    let mut missing = Vec::new();

    println!("[xtask] local audio fixtures in {}", fixtures_dir.display());

    for pair in &pairs {
        let audio_exists = Path::new(&pair.audio_path).exists();
        let transcript_exists = Path::new(&pair.transcript_path).exists();

        if !audio_exists || !transcript_exists {
            missing.push(pair.id);
            println!(
                "- test_{}: missing (audio={}, transcript={})",
                pair.id, audio_exists, transcript_exists
            );
            continue;
        }

        let transcript_preview = transcript_preview(&pair.transcript_path)?;
        println!(
            "- test_{}\n  audio: {}\n  transcript: {}\n  text: {}",
            pair.id, pair.audio_path, pair.transcript_path, transcript_preview
        );
    }

    if !missing.is_empty() {
        bail!(
            "missing required fixture pairs for test ids: {}",
            join_ids(&missing)
        );
    }

    Ok(())
}

pub fn ensure_required_fixtures_exist(fixtures_dir: &Path) -> Result<()> {
    let pairs = required_fixture_pairs(fixtures_dir);
    let mut missing = Vec::new();

    for pair in pairs {
        if !Path::new(&pair.audio_path).exists() || !Path::new(&pair.transcript_path).exists() {
            missing.push(pair.id);
        }
    }

    if missing.is_empty() {
        return Ok(());
    }

    bail!(
        "missing required fixture pairs for test ids: {}",
        join_ids(&missing)
    )
}

pub fn fixture_ids() -> &'static [u8] {
    &FIXTURE_IDS
}

fn required_fixture_pairs(fixtures_dir: &Path) -> Vec<FixturePair> {
    FIXTURE_IDS
        .iter()
        .map(|id| FixturePair {
            id: *id,
            audio_path: fixtures_dir
                .join(format!("test_{id}.mp3"))
                .display()
                .to_string(),
            transcript_path: fixtures_dir
                .join(format!("test_{id}.txt"))
                .display()
                .to_string(),
        })
        .collect()
}

fn transcript_preview(path: &str) -> Result<String> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read transcript file '{}'", path))?;

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok("<empty>".to_owned());
    }

    let collapsed = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    let preview = if collapsed.len() > 120 {
        format!("{}...", &collapsed[..120])
    } else {
        collapsed
    };

    Ok(preview)
}

fn join_ids(ids: &[u8]) -> String {
    ids.iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
