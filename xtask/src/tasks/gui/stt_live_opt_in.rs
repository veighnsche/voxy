use std::{env, time::Duration};

use anyhow::{bail, Result};
use clap::Args;

use crate::{tasks::gui::common, workspace};

const LIVE_GUARD_ENV: &str = "VOXY_E2E_LIVE_STT";

#[derive(Debug, Clone, Args)]
pub struct SttLiveOptInArgs {
    #[arg(long, default_value_t = 400)]
    pub startup_ms: u64,
    #[arg(long, default_value_t = 7_000)]
    pub auto_close_ms: u64,
    #[arg(long, default_value_t = 15_000)]
    pub timeout_ms: u64,
    #[arg(long, default_value_t = 1)]
    pub fixture_id: u8,
    #[arg(long, default_value_t = false)]
    pub no_build: bool,
}

pub fn run(args: SttLiveOptInArgs) -> Result<()> {
    if !env_flag_enabled(LIVE_GUARD_ENV) {
        bail!("live STT e2e is opt-in; set {LIVE_GUARD_ENV}=1 to run this command intentionally");
    }

    let root = workspace::root();

    if !args.no_build {
        common::build_gui(&root)?;
    }

    let app_id = common::make_app_id("stt-live-opt-in");
    let env = vec![
        ("VOXY_STT_BACKEND".to_owned(), "openai_api".to_owned()),
        ("VOXY_TRACE_PIPELINE".to_owned(), "1".to_owned()),
        ("VOXY_SMOKE_INJECT_RECORD_FLOW".to_owned(), "1".to_owned()),
        (
            "VOXY_SMOKE_RECORD_FIXTURE_ID".to_owned(),
            args.fixture_id.to_string(),
        ),
        (
            "VOXY_SMOKE_AUTO_CLOSE_MS".to_owned(),
            args.auto_close_ms.to_string(),
        ),
    ];
    let mut child = common::spawn_gui(&root, &app_id, &env)?;

    common::ensure_not_exited_early(&mut child, Duration::from_millis(args.startup_ms))?;

    let status = common::wait_for_exit(&mut child, Duration::from_millis(args.timeout_ms))?;
    if !status.success() {
        bail!("voxy-app stt-live-opt-in run exited with non-success status: {status}");
    }

    println!("[xtask] stt-live-opt-in check passed: {status}");
    Ok(())
}

fn env_flag_enabled(name: &str) -> bool {
    env::var(name)
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}
