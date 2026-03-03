use std::time::Duration;

use anyhow::{bail, Result};
use clap::Args;

use crate::{tasks::gui::common, workspace};

#[derive(Debug, Clone, Args)]
pub struct SttFixtureSmokeArgs {
    #[arg(long, default_value_t = 250)]
    pub startup_ms: u64,
    #[arg(long, default_value_t = 2200)]
    pub auto_close_ms: u64,
    #[arg(long, default_value_t = 7000)]
    pub timeout_ms: u64,
    #[arg(long, default_value_t = 1)]
    pub fixture_id: u8,
    #[arg(long, default_value_t = false)]
    pub no_build: bool,
}

pub fn run(args: SttFixtureSmokeArgs) -> Result<()> {
    let root = workspace::root();

    if !args.no_build {
        common::build_gui(&root)?;
    }

    let app_id = common::make_app_id("stt-fixture-smoke");
    let env = vec![
        ("VOXY_STT_BACKEND".to_owned(), "dummy".to_owned()),
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
        bail!("voxy-app stt-fixture-smoke run exited with non-success status: {status}");
    }

    println!("[xtask] stt-fixture-smoke check passed: {status}");
    Ok(())
}
