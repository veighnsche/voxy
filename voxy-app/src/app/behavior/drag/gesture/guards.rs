use std::{env, fs, os::fd::AsRawFd, os::unix::net::UnixStream, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DragMathMode {
    LegacyIncremental,
    PointerAnchor,
}

impl DragMathMode {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::LegacyIncremental => "legacy",
            Self::PointerAnchor => "anchor",
        }
    }
}

pub(super) fn detect_drag_math_mode() -> DragMathMode {
    if let Some(mode) = drag_math_mode_override() {
        return mode;
    }

    let compositor_name = detect_wayland_compositor_name();
    drag_math_mode_for_compositor_name(compositor_name.as_deref())
}

fn drag_math_mode_override() -> Option<DragMathMode> {
    let raw = env::var("VOXY_DRAG_MATH").ok()?;
    let value = raw.trim().to_ascii_lowercase();
    match value.as_str() {
        "legacy" | "kde" | "incremental" => Some(DragMathMode::LegacyIncremental),
        "anchor" | "niri" | "pointer" => Some(DragMathMode::PointerAnchor),
        _ => None,
    }
}

fn drag_math_mode_for_compositor_name(compositor_name: Option<&str>) -> DragMathMode {
    let Some(name) = compositor_name else {
        return DragMathMode::LegacyIncremental;
    };

    let normalized = name.to_ascii_lowercase();
    if normalized.contains("niri") {
        DragMathMode::PointerAnchor
    } else {
        DragMathMode::LegacyIncremental
    }
}

fn detect_wayland_compositor_name() -> Option<String> {
    let socket_path = detect_wayland_socket_path()?;
    let stream = UnixStream::connect(&socket_path).ok()?;
    let pid = peer_pid(stream.as_raw_fd())?;

    read_proc_comm(pid).or_else(|| read_proc_exe_name(pid))
}

fn detect_wayland_socket_path() -> Option<PathBuf> {
    let wayland_display = env::var("WAYLAND_DISPLAY").ok()?;
    let display_path = PathBuf::from(&wayland_display);
    if display_path.is_absolute() {
        return Some(display_path);
    }

    let runtime_dir = env::var("XDG_RUNTIME_DIR").ok()?;
    Some(PathBuf::from(runtime_dir).join(display_path))
}

fn peer_pid(fd: std::os::fd::RawFd) -> Option<u32> {
    let mut ucred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut ucred as *mut libc::ucred as *mut libc::c_void,
            &mut len,
        )
    };
    if rc != 0 || len as usize != std::mem::size_of::<libc::ucred>() || ucred.pid <= 0 {
        return None;
    }

    Some(ucred.pid as u32)
}

fn read_proc_comm(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/comm");
    let content = fs::read_to_string(path).ok()?;
    let name = content.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

fn read_proc_exe_name(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/exe");
    let target = fs::read_link(path).ok()?;
    target
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{drag_math_mode_for_compositor_name, DragMathMode};

    #[test]
    fn niri_uses_anchor_mode() {
        assert_eq!(
            drag_math_mode_for_compositor_name(Some("niri")),
            DragMathMode::PointerAnchor
        );
        assert_eq!(
            drag_math_mode_for_compositor_name(Some("Niri")),
            DragMathMode::PointerAnchor
        );
    }

    #[test]
    fn kwin_and_unknown_default_to_legacy_mode() {
        assert_eq!(
            drag_math_mode_for_compositor_name(Some("kwin_wayland")),
            DragMathMode::LegacyIncremental
        );
        assert_eq!(
            drag_math_mode_for_compositor_name(Some("sway")),
            DragMathMode::LegacyIncremental
        );
        assert_eq!(
            drag_math_mode_for_compositor_name(None),
            DragMathMode::LegacyIncremental
        );
    }
}
