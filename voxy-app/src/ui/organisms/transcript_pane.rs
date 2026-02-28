use gtk4 as gtk;
use gtk4::{prelude::*, ScrolledWindow, TextBuffer, TextView};

#[derive(Clone)]
pub struct TranscriptPane {
    pub container: ScrolledWindow,
    pub text_buffer: TextBuffer,
}

pub fn build() -> TranscriptPane {
    let text_view = TextView::new();
    text_view.set_vexpand(true);
    text_view.set_hexpand(true);
    text_view.set_wrap_mode(gtk::WrapMode::WordChar);
    text_view.set_editable(true);

    let text_buffer = text_view.buffer();

    let container = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&text_view)
        .build();

    TranscriptPane {
        container,
        text_buffer,
    }
}
