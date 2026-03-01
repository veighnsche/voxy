use gtk4::{
    prelude::*, Application, ApplicationWindow, Box as GtkBox, Orientation, Overlay, Stack,
};

use crate::ui::{
    atoms,
    organisms::{
        self, control_bar::ControlBar, error_banner::ErrorBanner, footer_status::FooterStatus,
        transcript_pane::TranscriptPane,
    },
};

#[derive(Clone)]
pub struct MainWindowTemplate {
    pub window: ApplicationWindow,
    pub control_bar: ControlBar,
    pub error_banner: ErrorBanner,
    pub transcript_pane: TranscriptPane,
    pub settings_pane: organisms::settings_pane::SettingsPane,
    pub content_stack: Stack,
    pub footer_status: FooterStatus,
    pub recording_frame: atoms::recording_frame::RecordingFrame,
    pub resize_handle: GtkBox,
}

pub fn build(app: &Application) -> MainWindowTemplate {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Voxy (Scaffold)")
        .default_width(360)
        .default_height(420)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);

    let control_bar = organisms::control_bar::build();
    let error_banner = organisms::error_banner::build();
    let transcript_pane = organisms::transcript_pane::build();
    let settings_pane = organisms::settings_pane::build();
    let content_stack = Stack::new();
    content_stack.set_hexpand(true);
    content_stack.set_vexpand(true);
    content_stack.add_named(&transcript_pane.container, Some("transcript"));
    content_stack.add_named(&settings_pane.container, Some("settings"));
    content_stack.set_visible_child_name("transcript");
    let footer_status = organisms::footer_status::build();
    let recording_frame = atoms::recording_frame::build();
    let resize_handle = atoms::resize_handle::build();

    root.append(&control_bar.container);
    root.append(&error_banner.revealer);
    root.append(&content_stack);
    root.append(&footer_status.container);

    let overlay = Overlay::new();
    overlay.set_child(Some(&root));
    overlay.add_overlay(&recording_frame.area);
    overlay.add_overlay(&resize_handle);

    window.set_child(Some(&overlay));

    MainWindowTemplate {
        window,
        control_bar,
        error_banner,
        transcript_pane,
        settings_pane,
        content_stack,
        footer_status,
        recording_frame,
        resize_handle,
    }
}
