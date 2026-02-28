use std::cell::Cell;

use gtk4::Application;

pub mod atoms;
pub mod molecules;
pub mod organisms;
pub mod pages;
pub mod templates;
pub mod view_model;

pub use pages::voxy_window_page::Widgets;
pub use view_model::ViewModel;

pub fn build(app: &Application) -> Widgets {
    pages::voxy_window_page::build(app)
}

pub fn render(widgets: &Widgets, view_model: &ViewModel, applying_text_update: &Cell<bool>) {
    pages::voxy_window_page::render(widgets, view_model, applying_text_update)
}
