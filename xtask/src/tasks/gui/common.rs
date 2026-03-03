use std::{
    io::Read,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};

use crate::workspace;

const GUI_TEST_THEME: &str = "Adwaita";

pub fn build_gui(root: &Path) -> Result<()> {
    println!("[xtask] building voxy-app");

    let status = Command::new("cargo")
        .args(["build", "-p", "voxy-app"])
        .current_dir(root)
        .status()
        .context("failed to run cargo build for voxy-app")?;

    if !status.success() {
        bail!("cargo build -p voxy-app failed with status {status}");
    }

    Ok(())
}

pub fn make_app_id(tag: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    format!("com.vince.voxy.{tag}.t{millis}")
}

pub fn spawn_gui(root: &Path, app_id: &str, extra_env: &[(String, String)]) -> Result<Child> {
    spawn_gui_inner(root, app_id, extra_env, false)
}

pub fn spawn_gui_captured(
    root: &Path,
    app_id: &str,
    extra_env: &[(String, String)],
) -> Result<Child> {
    spawn_gui_inner(root, app_id, extra_env, true)
}

pub fn collect_captured_output(child: &mut Child) -> Result<String> {
    let mut output = String::new();

    if let Some(mut stdout) = child.stdout.take() {
        let mut stdout_text = String::new();
        stdout
            .read_to_string(&mut stdout_text)
            .context("failed to read captured stdout")?;
        output.push_str(&stdout_text);
    }

    if let Some(mut stderr) = child.stderr.take() {
        let mut stderr_text = String::new();
        stderr
            .read_to_string(&mut stderr_text)
            .context("failed to read captured stderr")?;

        if !stderr_text.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str("[stderr]\n");
            output.push_str(&stderr_text);
        }
    }

    Ok(output)
}

fn spawn_gui_inner(
    root: &Path,
    app_id: &str,
    extra_env: &[(String, String)],
    capture_output: bool,
) -> Result<Child> {
    let binary = workspace::voxy_app_binary(root);
    if !binary.exists() {
        bail!("voxy-app binary not found at {}", binary.display());
    }

    println!(
        "[xtask] launching {} with app_id={app_id}",
        binary.display()
    );

    let mut command = Command::new(&binary);
    command
        .current_dir(root)
        .env("VOXY_APP_ID", app_id)
        .env("VOXY_NON_UNIQUE", "1")
        // Keep GUI smoke checks deterministic across host themes.
        .env("GTK_THEME", GUI_TEST_THEME);

    for (key, value) in extra_env {
        command.env(key, value);
    }

    if capture_output {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    }

    command
        .spawn()
        .with_context(|| format!("failed to spawn {}", binary.display()))
}

pub fn ensure_not_exited_early(child: &mut Child, startup_wait: Duration) -> Result<()> {
    thread::sleep(startup_wait);

    if let Some(status) = child.try_wait().context("failed to poll voxy-app")? {
        bail!("voxy-app exited too early: {status}");
    }

    Ok(())
}

pub fn wait_for_exit(child: &mut Child, timeout: Duration) -> Result<ExitStatus> {
    let start = Instant::now();

    loop {
        if let Some(status) = child.try_wait().context("failed to poll child process")? {
            return Ok(status);
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(anyhow!(
                "voxy-app did not exit within {}ms",
                timeout.as_millis()
            ));
        }

        thread::sleep(Duration::from_millis(50));
    }
}

pub fn send_termination_signal(child: &mut Child) -> Result<()> {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        println!("[xtask] sending SIGTERM to pid {pid}");

        // SAFETY: libc::kill is called with a valid child pid created by std::process::Command.
        let result = unsafe { libc::kill(pid, libc::SIGTERM) };
        if result != 0 {
            return Err(std::io::Error::last_os_error())
                .context("failed to send SIGTERM to voxy-app");
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        println!("[xtask] sending kill signal to process");
        child.kill().context("failed to terminate voxy-app")?;
        Ok(())
    }
}
