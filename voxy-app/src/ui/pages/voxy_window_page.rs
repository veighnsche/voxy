use std::cell::Cell;

use gtk4::{prelude::*, Application, ApplicationWindow, Button, Label, Revealer, TextBuffer};

use crate::ui::{molecules, organisms, templates, ViewModel};

#[derive(Clone)]
pub struct Widgets {
    pub window: ApplicationWindow,
    pub mic_button: Button,
    pub reset_button: Button,
    pub copy_button: Button,
    pub close_button: Button,
    pub text_buffer: TextBuffer,
    pub state_badge: Label,
    pub recording_indicator: molecules::recording_indicator::RecordingIndicator,
    pub error_revealer: Revealer,
    pub error_message_label: Label,
}

pub fn build(app: &Application) -> Widgets {
    let template = templates::main_window_template::build(app);

    Widgets {
        window: template.window,
        mic_button: template.control_bar.actions.mic_button,
        reset_button: template.control_bar.actions.reset_button,
        copy_button: template.control_bar.actions.copy_button,
        close_button: template.control_bar.actions.close_button,
        text_buffer: template.transcript_pane.text_buffer,
        state_badge: template.footer_status.state_badge,
        recording_indicator: template.footer_status.recording_indicator,
        error_revealer: template.error_banner.revealer,
        error_message_label: template.error_banner.message,
    }
}

pub fn render(widgets: &Widgets, view_model: &ViewModel, applying_text_update: &Cell<bool>) {
    applying_text_update.set(true);
    widgets.text_buffer.set_text(&view_model.text);
    applying_text_update.set(false);

    crate::ui::atoms::mic_button::render(&widgets.mic_button, view_model.mic_on);
    crate::ui::atoms::state_badge::render(&widgets.state_badge, &view_model.state_badge_text);
    molecules::recording_indicator::render(&widgets.recording_indicator, view_model.mic_on);

    let error_banner = organisms::error_banner::ErrorBanner {
        revealer: widgets.error_revealer.clone(),
        message: widgets.error_message_label.clone(),
    };
    organisms::error_banner::render(&error_banner, view_model.error_message.as_deref());
}
