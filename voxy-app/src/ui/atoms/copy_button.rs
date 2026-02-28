use gtk4::{prelude::*, Box as GtkBox, Button, Image, Label, Orientation};

pub fn build() -> Button {
    let button = Button::new();
    let content = GtkBox::new(Orientation::Horizontal, 6);
    let icon = Image::from_icon_name("edit-copy-symbolic");
    let label = Label::new(Some("Copy"));
    content.append(&icon);
    content.append(&label);
    button.set_child(Some(&content));
    button.set_tooltip_text(Some("Copy transcript to clipboard"));
    button
}
