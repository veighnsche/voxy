use gtk4::{gdk, prelude::*, Align, Box as GtkBox, Label, Orientation};

pub fn build() -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.add_css_class("resize-handle");
    container.set_size_request(20, 20);
    container.set_halign(Align::End);
    container.set_valign(Align::End);
    container.set_margin_end(4);
    container.set_margin_bottom(4);
    container.set_tooltip_text(Some("Drag to resize"));

    let glyph = Label::new(Some("◢"));
    glyph.add_css_class("dim-label");
    glyph.set_can_target(false);
    container.append(&glyph);

    if let Some(cursor) = gdk::Cursor::from_name("se-resize", None) {
        container.set_cursor(Some(&cursor));
    }

    container
}
