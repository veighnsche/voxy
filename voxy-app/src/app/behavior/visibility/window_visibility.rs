use gtk4::{prelude::*, ApplicationWindow};

pub fn show_window(window: &ApplicationWindow) {
    window.present();
}

pub fn hide_window(window: &ApplicationWindow) {
    window.set_visible(false);
}
