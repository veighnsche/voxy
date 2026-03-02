use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use voxy_core::{AppState, CoreModel};

use crate::ui::{self, ViewModel, Widgets};

pub fn render(
    widgets: &Widgets,
    model: &Rc<RefCell<CoreModel>>,
    applying_text_update: &Rc<Cell<bool>>,
) {
    let model = model.borrow();

    let view_model = build_view_model(&model);
    ui::render(widgets, &view_model, applying_text_update);
}

pub fn build_view_model(model: &CoreModel) -> ViewModel {
    let mic_on = matches!(model.app_state, AppState::Recording);

    let (state_text, app_error_message) = match &model.app_state {
        AppState::Idle => ("Idle".to_owned(), None),
        AppState::Recording => ("Recording".to_owned(), None),
        AppState::Processing => ("Processing".to_owned(), None),
        AppState::Error(message) => (format!("Error({message})"), Some(message.as_str())),
    };

    let error_message = model
        .runtime_error
        .as_ref()
        .cloned()
        .or_else(|| app_error_message.map(str::to_owned));

    ViewModel {
        text: model.buffer.full_text(),
        mic_on,
        settings_open: model.ui_prefs.settings_open,
        silence_timeout_seconds: model.ui_prefs.silence_auto_stop_seconds,
        vad_silence_ms: model.ui_prefs.vad_silence_duration_ms,
        state_badge_text: state_text,
        log_text: model.log_line.clone(),
        error_message,
    }
}
