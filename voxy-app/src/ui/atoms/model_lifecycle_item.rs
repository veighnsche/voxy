use std::{cell::Cell, rc::Rc};

use gtk4::{
    pango, prelude::*, Align, Box as GtkBox, Button, Image, Label, Orientation, Overlay, Popover,
    ProgressBar,
};
use voxy_models::InstallState;

#[derive(Clone)]
pub struct ModelLifecycleItem {
    pub button: Button,
    pub action_button: Button,
    popover: Popover,
    progress_bar: ProgressBar,
    state: Rc<Cell<InstallState>>,
    busy: Rc<Cell<bool>>,
    status_icon: Image,
    status_label: Label,
}

pub fn build(title: &str, subtitle: &str, model_id: &str) -> ModelLifecycleItem {
    let title_label = Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.set_ellipsize(pango::EllipsizeMode::End);

    let subtitle_label = Label::new(Some(subtitle));
    subtitle_label.set_xalign(0.0);
    subtitle_label.set_ellipsize(pango::EllipsizeMode::End);
    subtitle_label.add_css_class("dim-label");

    let text_stack = GtkBox::new(Orientation::Vertical, 2);
    text_stack.set_hexpand(true);
    text_stack.set_halign(Align::Fill);
    text_stack.append(&title_label);
    text_stack.append(&subtitle_label);

    let status_icon = Image::from_icon_name("folder-download-symbolic");
    status_icon.set_pixel_size(16);
    status_icon.set_tooltip_text(Some("Not downloaded"));

    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_hexpand(true);
    row.set_halign(Align::Fill);
    row.set_margin_top(4);
    row.set_margin_bottom(4);
    row.set_margin_start(6);
    row.set_margin_end(6);
    row.append(&status_icon);
    row.append(&text_stack);

    let progress_bar = ProgressBar::new();
    progress_bar.set_hexpand(true);
    progress_bar.set_halign(Align::Fill);
    progress_bar.set_valign(Align::Fill);
    progress_bar.set_fraction(0.0);
    progress_bar.set_visible(false);
    progress_bar.set_opacity(0.35);
    progress_bar.set_can_target(false);

    let overlay = Overlay::new();
    overlay.set_hexpand(true);
    overlay.set_halign(Align::Fill);
    overlay.set_child(Some(&progress_bar));
    overlay.add_overlay(&row);
    overlay.set_measure_overlay(&row, true);

    let button = Button::new();
    button.set_hexpand(true);
    button.set_halign(Align::Fill);
    button.set_can_focus(true);
    button.set_child(Some(&overlay));
    button.set_tooltip_text(Some(model_id));

    let popover = Popover::new();
    let popover_content = GtkBox::new(Orientation::Vertical, 6);
    popover_content.set_margin_top(8);
    popover_content.set_margin_bottom(8);
    popover_content.set_margin_start(8);
    popover_content.set_margin_end(8);

    let status_label = Label::new(Some("Not downloaded"));
    status_label.set_xalign(0.0);
    status_label.add_css_class("dim-label");

    let action_button = Button::with_label("Download");
    action_button.set_hexpand(true);
    action_button.set_halign(Align::Fill);

    popover_content.append(&status_label);
    popover_content.append(&action_button);
    popover.set_child(Some(&popover_content));
    popover.set_autohide(true);
    popover.set_has_arrow(true);
    popover.set_parent(&button);

    let busy = Rc::new(Cell::new(false));
    let busy_for_click = Rc::clone(&busy);
    let popover_for_click = popover.clone();
    button.connect_clicked(move |_| {
        if busy_for_click.get() {
            return;
        }
        if popover_for_click.is_visible() {
            popover_for_click.popdown();
        } else {
            popover_for_click.popup();
        }
    });

    let item = ModelLifecycleItem {
        button,
        action_button,
        popover,
        progress_bar,
        state: Rc::new(Cell::new(InstallState::NotDownloaded)),
        busy,
        status_icon,
        status_label,
    };
    item.set_state(InstallState::NotDownloaded);
    item
}

impl ModelLifecycleItem {
    pub fn set_state(&self, state: InstallState) {
        self.state.set(state);
        let (icon_name, icon_tooltip, popover_label, action_label) = match state {
            InstallState::NotDownloaded => (
                "folder-download-symbolic",
                "Available to download",
                "Not downloaded",
                "Download",
            ),
            InstallState::Downloaded => (
                "emblem-ok-symbolic",
                "Downloaded locally",
                "Downloaded",
                "Remove",
            ),
        };

        self.status_icon.set_icon_name(Some(icon_name));
        self.status_icon.set_tooltip_text(Some(icon_tooltip));
        if !self.busy.get() {
            self.status_label.set_label(popover_label);
            self.action_button.set_label(action_label);
        }
    }

    pub fn is_busy(&self) -> bool {
        self.busy.get()
    }

    pub fn set_busy(&self, busy: bool) {
        self.busy.set(busy);
        self.button.set_sensitive(!busy);
        self.action_button.set_sensitive(!busy);
        if busy {
            self.status_label.set_label("Working...");
            self.action_button.set_label("Working...");
            self.set_progress(Some(0.0));
        } else {
            self.set_progress(None);
            let state = self.state.get();
            self.set_state(state);
        }
    }

    pub fn set_progress(&self, progress: Option<f32>) {
        match progress {
            Some(value) => {
                self.progress_bar.set_visible(true);
                self.progress_bar.set_fraction(value.clamp(0.0, 1.0) as f64);
            }
            None => {
                self.progress_bar.set_fraction(0.0);
                self.progress_bar.set_visible(false);
            }
        }
    }

    pub fn popdown(&self) {
        self.popover.popdown();
    }
}
