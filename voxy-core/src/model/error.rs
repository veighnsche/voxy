use super::{CoreCommand, CoreModel};

impl CoreModel {
    pub(super) fn reduce_runtime_error(&mut self, message: String) -> Vec<CoreCommand> {
        self.log_line = format!("Error: {message}");
        self.runtime_error = Some(message);
        Vec::new()
    }

    pub(super) fn reduce_error_cleared(&mut self) -> Vec<CoreCommand> {
        self.runtime_error = None;
        Vec::new()
    }
}
