use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should live under workspace root")
        .to_path_buf()
}

pub fn voxy_app_binary(root: &Path) -> PathBuf {
    let path = root.join("target").join("debug").join("voxy-app");

    #[cfg(windows)]
    {
        let mut path = path;
        path.set_extension("exe");
        return path;
    }

    #[cfg(not(windows))]
    path
}
