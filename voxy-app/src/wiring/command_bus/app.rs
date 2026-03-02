use gtk4::prelude::ApplicationExt;

use crate::{diagnostics::pipeline_trace, tray};

use super::CommandBus;

impl CommandBus {
    pub(super) fn handle_quit_application(&self) {
        pipeline_trace::log("command", "QuitApplication");
        tray::shutdown();
        self.app.quit();
    }
}
