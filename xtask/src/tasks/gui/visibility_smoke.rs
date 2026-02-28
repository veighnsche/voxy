use std::time::Duration;

use anyhow::{bail, Result};
use clap::Args;

use crate::{tasks::gui::common, workspace};

#[derive(Debug, Clone, Args)]
pub struct VisibilitySmokeArgs {
    #[arg(long, default_value_t = 250)]
    pub startup_ms: u64,
    #[arg(long, default_value_t = 1200)]
    pub auto_close_ms: u64,
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u64,
    #[arg(long, default_value_t = false)]
    pub no_build: bool,
}

pub fn run(args: VisibilitySmokeArgs) -> Result<()> {
    let root = workspace::root();

    if !args.no_build {
        common::build_gui(&root)?;
    }

    let app_id = common::make_app_id("visibility-smoke");
    let env = vec![
        (
            "VOXY_SMOKE_INJECT_VISIBILITY_TOGGLE".to_owned(),
            "1".to_owned(),
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
        bail!("voxy-app visibility-smoke run exited with non-success status: {status}");
    }

    println!("[xtask] visibility-smoke check passed: {status}");
    Ok(())
}
