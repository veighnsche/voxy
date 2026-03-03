use std::env;

use gtk4::{gio::ApplicationFlags, Application};

const DEFAULT_APP_ID: &str = "com.vince.voxy";

pub fn build_application() -> Application {
    Application::builder()
        .application_id(resolve_app_id())
        .flags(resolve_flags())
        .build()
}

fn resolve_app_id() -> String {
    env::var("VOXY_APP_ID")
        .ok()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_APP_ID.to_owned())
}

fn resolve_flags() -> ApplicationFlags {
    if env_flag_enabled("VOXY_NON_UNIQUE") {
        ApplicationFlags::NON_UNIQUE
    } else {
        ApplicationFlags::empty()
    }
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
