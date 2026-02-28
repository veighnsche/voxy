use gtk4::{
    prelude::{DisplayExt, WidgetExt},
    ApplicationWindow,
};

pub fn copy_text_to_clipboard(window: &ApplicationWindow, text: &str) {
    let display = window.display();
    let clipboard = display.clipboard();
    clipboard.set_text(text);
}
