use std::time::Duration;

use anyhow::Result;
use clap::Args;

use crate::{tasks::gui::common, workspace};

#[derive(Debug, Clone, Args)]
pub struct SmokeArgs {
    #[arg(long, default_value_t = 1500)]
    pub startup_ms: u64,
    #[arg(long, default_value_t = 5000)]
    pub shutdown_timeout_ms: u64,
    #[arg(long, default_value_t = false)]
    pub no_build: bool,
}

pub fn run(args: SmokeArgs) -> Result<()> {
    let root = workspace::root();

    if !args.no_build {
        common::build_gui(&root)?;
    }

    let app_id = common::make_app_id("smoke");
    let mut child = common::spawn_gui(&root, &app_id, &[])?;

    common::ensure_not_exited_early(&mut child, Duration::from_millis(args.startup_ms))?;
    common::send_termination_signal(&mut child)?;

    let status =
        common::wait_for_exit(&mut child, Duration::from_millis(args.shutdown_timeout_ms))?;
    println!("[xtask] voxy-app exited after signal: {status}");

    Ok(())
}
