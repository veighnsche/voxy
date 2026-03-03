use std::{
    env,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use gtk4::{prelude::*, ApplicationWindow};
use tokio::sync::mpsc;
use voxy_core::AppEvent;

const READY_MARKER_ENV: &str = "VOXY_SMOKE_MARK_READY";
const WINDOW_CREATED_MARKER_ENV: &str = "VOXY_SMOKE_MARK_WINDOW_CREATED";
const AUTO_CLOSE_MS_ENV: &str = "VOXY_SMOKE_AUTO_CLOSE_MS";
const INJECT_RESET_ENV: &str = "VOXY_SMOKE_INJECT_RESET";
const INJECT_VISIBILITY_TOGGLE_ENV: &str = "VOXY_SMOKE_INJECT_VISIBILITY_TOGGLE";
const VISIBILITY_TOGGLE_COUNT_ENV: &str = "VOXY_SMOKE_VISIBILITY_TOGGLE_COUNT";
const INJECT_RECORD_FLOW_ENV: &str = "VOXY_SMOKE_INJECT_RECORD_FLOW";
const RECORD_FLOW_START_MS_ENV: &str = "VOXY_SMOKE_RECORD_START_MS";
const RECORD_FLOW_FIXTURE_MS_ENV: &str = "VOXY_SMOKE_RECORD_FIXTURE_MS";
const RECORD_FLOW_STOP_MS_ENV: &str = "VOXY_SMOKE_RECORD_STOP_MS";
const RECORD_FLOW_FIXTURE_ID_ENV: &str = "VOXY_SMOKE_RECORD_FIXTURE_ID";
const INJECT_WINDOW_CLOSE_ENV: &str = "VOXY_SMOKE_INJECT_WINDOW_CLOSE";
const WINDOW_CLOSE_MS_ENV: &str = "VOXY_SMOKE_WINDOW_CLOSE_MS";
const RESET_EMIT_DELAY_MS: u64 = 100;
const VISIBILITY_TOGGLE_EMIT_INTERVAL_MS: u64 = 140;
const DEFAULT_RECORD_FLOW_START_MS: u64 = 120;
const DEFAULT_RECORD_FLOW_FIXTURE_MS: u64 = 260;
const DEFAULT_RECORD_FLOW_STOP_MS: u64 = 1_000;
const DEFAULT_WINDOW_CLOSE_MS: u64 = 220;
static WINDOW_CREATED_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn install(window: &ApplicationWindow, event_tx: &mpsc::Sender<AppEvent>) {
    if env_flag_enabled(READY_MARKER_ENV) {
        println!("VOXY_SMOKE_READY");
    }

    if env_flag_enabled(INJECT_RESET_ENV) {
        schedule_event(
            event_tx.clone(),
            Duration::from_millis(RESET_EMIT_DELAY_MS),
            AppEvent::ResetRequested,
        );
    }

    if env_flag_enabled(INJECT_RECORD_FLOW_ENV) {
        let start_ms = env_u64(RECORD_FLOW_START_MS_ENV).unwrap_or(DEFAULT_RECORD_FLOW_START_MS);
        let fixture_ms =
            env_u64(RECORD_FLOW_FIXTURE_MS_ENV).unwrap_or(DEFAULT_RECORD_FLOW_FIXTURE_MS);
        let stop_ms = env_u64(RECORD_FLOW_STOP_MS_ENV).unwrap_or(DEFAULT_RECORD_FLOW_STOP_MS);
        let fixture_id = env_u8(RECORD_FLOW_FIXTURE_ID_ENV).unwrap_or(1);

        schedule_event(
            event_tx.clone(),
            Duration::from_millis(start_ms),
            AppEvent::MicToggled,
        );
        schedule_event(
            event_tx.clone(),
            Duration::from_millis(fixture_ms),
            AppEvent::FixtureInjectRequested(fixture_id),
        );
        schedule_event(
            event_tx.clone(),
            Duration::from_millis(stop_ms),
            AppEvent::MicToggled,
        );
    }

    if let Some(delay_ms) = env_u64(AUTO_CLOSE_MS_ENV) {
        let window = window.clone();
        gtk4::glib::timeout_add_local(Duration::from_millis(delay_ms), move || {
            if let Some(app) = window.application() {
                app.quit();
            }
            gtk4::glib::ControlFlow::Break
        });
    }

    if env_flag_enabled(INJECT_WINDOW_CLOSE_ENV) {
        let delay_ms = env_u64(WINDOW_CLOSE_MS_ENV).unwrap_or(DEFAULT_WINDOW_CLOSE_MS);
        let window = window.clone();
        gtk4::glib::timeout_add_local(Duration::from_millis(delay_ms), move || {
            window.close();
            gtk4::glib::ControlFlow::Break
        });
    }
}

pub fn install_visibility_toggle_injector(mut on_toggle: impl FnMut() + 'static) {
    if env_flag_enabled(INJECT_VISIBILITY_TOGGLE_ENV) {
        let mut remaining = env_u32(VISIBILITY_TOGGLE_COUNT_ENV).unwrap_or(1).max(1);
        gtk4::glib::timeout_add_local(
            Duration::from_millis(VISIBILITY_TOGGLE_EMIT_INTERVAL_MS),
            move || {
                on_toggle();
                remaining -= 1;

                if remaining == 0 {
                    gtk4::glib::ControlFlow::Break
                } else {
                    gtk4::glib::ControlFlow::Continue
                }
            },
        );
    }
}

pub fn mark_window_created() {
    if env_flag_enabled(WINDOW_CREATED_MARKER_ENV) {
        let count = WINDOW_CREATED_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
        println!("VOXY_SMOKE_WINDOW_CREATED:{count}");
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

fn env_u64(name: &str) -> Option<u64> {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
}

fn env_u32(name: &str) -> Option<u32> {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
}

fn env_u8(name: &str) -> Option<u8> {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u8>().ok())
}

fn schedule_event(event_tx: mpsc::Sender<AppEvent>, delay: Duration, event: AppEvent) {
    gtk4::glib::timeout_add_local(delay, move || {
        let _ = event_tx.try_send(event.clone());
        gtk4::glib::ControlFlow::Break
    });
}
