use gtk4::prelude::GtkWindowExt;

use crate::{
    app::behavior::{self, surface::layer_shell::MonitorCycleOutcome},
    diagnostics::pipeline_trace,
};

use super::CommandBus;

impl CommandBus {
    pub(super) fn handle_show_window(&self) {
        behavior::visibility::window_visibility::show_window(&self.window);
    }

    pub(super) fn handle_hide_window(&self) {
        behavior::visibility::window_visibility::hide_window(&self.window);
    }

    pub(super) fn handle_resize_window(&self, width: i32, height: i32) {
        self.window.set_default_size(width, height);
    }

    pub(super) fn handle_move_window_to_next_screen(&self) {
        match self
            .layer_shell_backend
            .move_window_to_next_monitor(&self.window)
        {
            Ok(MonitorCycleOutcome::Moved {
                from_index,
                to_index,
                monitor_count,
            }) => {
                self.emit_log_message(format!(
                    "Moved to screen {}/{}",
                    to_index + 1,
                    monitor_count
                ));
                pipeline_trace::log(
                    "command",
                    format!(
                        "MoveWindowToNextScreen moved from={} to={} total={}",
                        from_index + 1,
                        to_index + 1,
                        monitor_count
                    ),
                );
            }
            Ok(MonitorCycleOutcome::SingleMonitor) => {
                self.emit_log_message("Only one screen detected; nothing to move");
                pipeline_trace::log("command", "MoveWindowToNextScreen single_monitor");
            }
            Err(error) => {
                pipeline_trace::log("command", format!("MoveWindowToNextScreen error={error}"));
                self.emit_runtime_error(error);
            }
        }
    }

    pub(super) fn handle_copy_text_to_clipboard(&self, text: String) {
        behavior::system::clipboard::copy_text_to_clipboard(&self.window, &text);
        self.emit_log_message("Transcript copied to clipboard");
    }
}
