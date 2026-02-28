use std::time::Duration;

use anyhow::{bail, Result};
use clap::Args;

use crate::{tasks::gui::common, workspace};

const WINDOW_CREATED_MARKER: &str = "VOXY_SMOKE_WINDOW_CREATED:";

#[derive(Debug, Clone, Args)]
pub struct VisibilityWindowGuardArgs {
    #[arg(long, default_value_t = 250)]
    pub startup_ms: u64,
    #[arg(long, default_value_t = 1200)]
    pub auto_close_ms: u64,
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u64,
    #[arg(long, default_value_t = 3)]
    pub visibility_toggle_count: u32,
    #[arg(long, default_value_t = false)]
    pub no_build: bool,
}

pub fn run(args: VisibilityWindowGuardArgs) -> Result<()> {
    let root = workspace::root();

    if !args.no_build {
        common::build_gui(&root)?;
    }

    let app_id = common::make_app_id("visibility-window-guard");
    let env = vec![
        (
            "VOXY_SMOKE_INJECT_VISIBILITY_TOGGLE".to_owned(),
            "1".to_owned(),
        ),
        (
            "VOXY_SMOKE_VISIBILITY_TOGGLE_COUNT".to_owned(),
            args.visibility_toggle_count.max(1).to_string(),
        ),
        (
            "VOXY_SMOKE_AUTO_CLOSE_MS".to_owned(),
            args.auto_close_ms.to_string(),
        ),
        ("VOXY_SMOKE_MARK_WINDOW_CREATED".to_owned(), "1".to_owned()),
    ];

    let mut child = common::spawn_gui_captured(&root, &app_id, &env)?;

    common::ensure_not_exited_early(&mut child, Duration::from_millis(args.startup_ms))?;

    let status = common::wait_for_exit(&mut child, Duration::from_millis(args.timeout_ms))?;
    let output = common::collect_captured_output(&mut child)?;
    if !status.success() {
        bail!(
            "voxy-app visibility-window-guard run exited with non-success status: {status}\n{output}"
        );
    }

    let creation_markers = output
        .lines()
        .filter(|line| line.trim_start().starts_with(WINDOW_CREATED_MARKER))
        .count();

    if creation_markers != 1 {
        bail!(
            "visibility-window-guard failed: expected exactly 1 window creation marker, got {creation_markers}\n{output}"
        );
    }

    println!(
        "[xtask] visibility-window-guard passed: window creation marker count={creation_markers}, status={status}"
    );
    Ok(())
}
