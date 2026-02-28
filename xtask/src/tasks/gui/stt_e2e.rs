use anyhow::{bail, Result};
use clap::Args;

use crate::tasks::{fixtures, gui::smoke};

#[derive(Debug, Clone, Args)]
pub struct SttE2eArgs {
    #[arg(long, default_value_t = false)]
    pub no_build: bool,
}

pub fn run(args: SttE2eArgs) -> Result<()> {
    if !env_flag_enabled("VOXY_E2E_LIVE") {
        bail!("stt-e2e is opt-in. Set VOXY_E2E_LIVE=1 and ensure API key env is configured");
    }

    fixtures::verify_audio::run(fixtures::verify_audio::VerifyAudioArgs {
        manifest: "tests/fixtures/audio/manifest.json".to_owned(),
    })?;

    smoke::run(smoke::SmokeArgs {
        startup_ms: 2000,
        shutdown_timeout_ms: 5000,
        no_build: args.no_build,
    })?;

    println!("[xtask] stt-e2e preflight passed (live STT execution stub)");
    Ok(())
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}
