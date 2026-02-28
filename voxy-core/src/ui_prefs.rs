#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPrefs {
    pub visible: bool,
    pub window_left: i32,
    pub window_top: i32,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            visible: true,
            window_left: 24,
            window_top: 24,
        }
    }
}
