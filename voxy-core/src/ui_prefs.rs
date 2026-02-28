#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPrefs {
    pub visible: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self { visible: true }
    }
}
