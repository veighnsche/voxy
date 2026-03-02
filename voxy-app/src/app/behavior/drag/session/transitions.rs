use super::DragSession;

impl DragSession {
    pub fn begin(&self, base_left: i32, base_top: i32) {
        self.active.set(true);
        self.base_left.set(base_left);
        self.base_top.set(base_top);
        self.last_position.set(None);
    }

    pub fn cancel(&self) {
        self.end();
    }

    pub fn end(&self) {
        self.active.set(false);
        self.last_position.set(None);
    }

    pub fn is_active(&self) -> bool {
        self.active.get()
    }
}
