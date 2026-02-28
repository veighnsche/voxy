use gtk4::{prelude::*, Application, ApplicationWindow, Box as GtkBox, Orientation};

use crate::ui::organisms::{
    self, control_bar::ControlBar, error_banner::ErrorBanner, footer_status::FooterStatus,
    transcript_pane::TranscriptPane,
};

#[derive(Clone)]
pub struct MainWindowTemplate {
    pub window: ApplicationWindow,
    pub control_bar: ControlBar,
    pub error_banner: ErrorBanner,
    pub transcript_pane: TranscriptPane,
    pub footer_status: FooterStatus,
}

pub fn build(app: &Application) -> MainWindowTemplate {
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

    let control_bar = organisms::control_bar::build();
    let error_banner = organisms::error_banner::build();
    let transcript_pane = organisms::transcript_pane::build();
    let footer_status = organisms::footer_status::build();

    root.append(&control_bar.container);
    root.append(&error_banner.revealer);
    root.append(&transcript_pane.container);
    root.append(&footer_status.container);

    window.set_child(Some(&root));

    MainWindowTemplate {
        window,
        control_bar,
        error_banner,
        transcript_pane,
        footer_status,
    }
}
