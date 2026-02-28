use std::cell::Cell;

#[derive(Default)]
pub struct DragSession {
    active: Cell<bool>,
    base_left: Cell<i32>,
    base_top: Cell<i32>,
}

impl DragSession {
    pub fn begin(&self, left: i32, top: i32) {
        self.active.set(true);
        self.base_left.set(left);
        self.base_top.set(top);
    }

    pub fn cancel(&self) {
        self.active.set(false);
    }

    pub fn end(&self) {
        self.active.set(false);
    }

    pub fn is_active(&self) -> bool {
        self.active.get()
    }

    pub fn position_for(&self, offset_x: f64, offset_y: f64) -> (i32, i32) {
        let left = ((self.base_left.get() as f64) + offset_x).round().max(0.0) as i32;
        let top = ((self.base_top.get() as f64) + offset_y).round().max(0.0) as i32;
        (left, top)
    }
}
