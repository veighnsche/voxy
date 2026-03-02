use super::{CoreCommand, CoreModel};

pub(super) const WINDOW_RESIZE_STEP: i32 = 40;
pub(super) const WINDOW_MIN_WIDTH: i32 = 280;
pub(super) const WINDOW_MIN_HEIGHT: i32 = 320;
pub(super) const WINDOW_MAX_WIDTH: i32 = 960;
pub(super) const WINDOW_MAX_HEIGHT: i32 = 1280;

impl CoreModel {
    pub(super) fn reduce_window_larger_requested(&mut self) -> Vec<CoreCommand> {
        self.resize_window_by(WINDOW_RESIZE_STEP, WINDOW_RESIZE_STEP)
    }

    pub(super) fn reduce_window_smaller_requested(&mut self) -> Vec<CoreCommand> {
        self.resize_window_by(-WINDOW_RESIZE_STEP, -WINDOW_RESIZE_STEP)
    }

    pub(super) fn reduce_window_resize_requested(
        &mut self,
        width: i32,
        height: i32,
    ) -> Vec<CoreCommand> {
        self.resize_window_to(width, height)
    }

    pub(super) fn reduce_window_move_to_next_screen_requested(&mut self) -> Vec<CoreCommand> {
        self.log_line = "Move to next screen requested".to_owned();
        vec![CoreCommand::MoveWindowToNextScreen]
    }

    pub(super) fn reduce_visibility_toggled(&mut self) -> Vec<CoreCommand> {
        self.ui_prefs.visible = !self.ui_prefs.visible;

        if self.ui_prefs.visible {
            vec![CoreCommand::ShowWindow]
        } else {
            vec![CoreCommand::HideWindow]
        }
    }

    pub(super) fn reduce_show_requested(&mut self) -> Vec<CoreCommand> {
        self.ui_prefs.visible = true;
        vec![CoreCommand::ShowWindow]
    }

    pub(super) fn reduce_hide_requested(&mut self) -> Vec<CoreCommand> {
        if !self.ui_prefs.visible {
            return Vec::new();
        }

        self.ui_prefs.visible = false;
        vec![CoreCommand::HideWindow]
    }

    fn resize_window_by(&mut self, width_delta: i32, height_delta: i32) -> Vec<CoreCommand> {
        let next_width = self.ui_prefs.window_width.saturating_add(width_delta);
        let next_height = self.ui_prefs.window_height.saturating_add(height_delta);
        self.set_window_size(next_width, next_height);
        self.log_line = format!(
            "Window resized to {}x{}",
            self.ui_prefs.window_width, self.ui_prefs.window_height
        );
        vec![CoreCommand::ResizeWindow {
            width: self.ui_prefs.window_width,
            height: self.ui_prefs.window_height,
        }]
    }

    fn resize_window_to(&mut self, width: i32, height: i32) -> Vec<CoreCommand> {
        self.set_window_size(width, height);
        vec![CoreCommand::ResizeWindow {
            width: self.ui_prefs.window_width,
            height: self.ui_prefs.window_height,
        }]
    }
}
