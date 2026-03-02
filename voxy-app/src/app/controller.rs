use std::sync::Arc;

use gtk4::prelude::*;

use crate::{app::lifecycle, wiring};

mod bootstrap;
mod event_processing;
mod input_meter_loop;
mod settings_sync;
mod ui_signals;

pub fn run() {
    let runtime = match wiring::runtime::build() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to create tokio runtime: {error}");
            return;
        }
    };
    let app = lifecycle::build_application();

    app.connect_activate(move |app| bootstrap::activate(app, Arc::clone(&runtime)));
    app.run();
}
