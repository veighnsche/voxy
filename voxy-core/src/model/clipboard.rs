use super::{CoreCommand, CoreModel};

impl CoreModel {
    pub(super) fn reduce_copy_requested(&mut self) -> Vec<CoreCommand> {
        self.log_line = "Copy requested".to_owned();
        vec![CoreCommand::CopyTextToClipboard(self.buffer.full_text())]
    }
}
