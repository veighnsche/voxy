use std::{cell::RefCell, rc::Rc};

use voxy_core::{to_error_state, CoreModel};

#[allow(dead_code)]
pub fn set_error(model: &Rc<RefCell<CoreModel>>, message: impl Into<String>) {
    model.borrow_mut().app_state = to_error_state(message);
}
