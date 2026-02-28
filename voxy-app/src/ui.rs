use std::cell::Cell;

use gtk::{
    prelude::*, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation,
    ScrolledWindow, TextBuffer, TextView,
};
use gtk4 as gtk;

#[derive(Clone)]
pub struct Widgets {
    pub window: ApplicationWindow,
    pub mic_button: Button,
    pub reset_button: Button,
    pub pin_button: Button,
    pub text_buffer: TextBuffer,
    pub status_label: Label,
}

pub struct ViewModel {
    pub text: String,
    pub mic_on: bool,
    pub pinned: bool,
    pub status_text: String,
}

pub fn build(app: &Application) -> Widgets {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Voxy (Scaffold)")
        .default_width(720)
        .default_height(420)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);

    let controls = GtkBox::new(Orientation::Horizontal, 8);

    let mic_button = Button::with_label("Mic: Off");
    let reset_button = Button::with_label("Reset");
    let pin_button = Button::with_label("Pin: Off");

    controls.append(&mic_button);
    controls.append(&reset_button);
    controls.append(&pin_button);

    let text_view = TextView::new();
    text_view.set_vexpand(true);
    text_view.set_hexpand(true);
    text_view.set_wrap_mode(gtk::WrapMode::WordChar);
    text_view.set_editable(true);

    let text_buffer = text_view.buffer();

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&text_view)
        .build();

    let status_label = Label::new(None);
    status_label.set_xalign(0.0);

    root.append(&controls);
    root.append(&scroll);
    root.append(&status_label);

    window.set_child(Some(&root));

    Widgets {
        window,
        mic_button,
        reset_button,
        pin_button,
        text_buffer,
        status_label,
    }
}

pub fn render(widgets: &Widgets, view_model: &ViewModel, applying_text_update: &Cell<bool>) {
    applying_text_update.set(true);
    widgets.text_buffer.set_text(&view_model.text);
    applying_text_update.set(false);

    widgets.mic_button.set_label(if view_model.mic_on {
        "Mic: On"
    } else {
        "Mic: Off"
    });
    widgets.pin_button.set_label(if view_model.pinned {
        "Pin: On"
    } else {
        "Pin: Off"
    });
    widgets.status_label.set_text(&view_model.status_text);
}
