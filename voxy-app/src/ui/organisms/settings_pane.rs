use gtk4::{
    prelude::*, Adjustment, Align, Box as GtkBox, Label, Orientation, ScrolledWindow, Separator,
    SpinButton,
};
use voxy_models::{InstallState, ManagedModel};

use crate::ui::atoms::model_lifecycle_item::{self, ModelLifecycleItem};

const MIN_TIMEOUT_SECONDS: f64 = 0.0;
const MAX_TIMEOUT_SECONDS: f64 = 600.0;

#[derive(Clone)]
pub struct ModelLifecycleControl {
    pub model: ManagedModel,
    pub item: ModelLifecycleItem,
}

#[derive(Clone)]
pub struct SettingsPane {
    pub container: ScrolledWindow,
    pub silence_timeout_seconds: SpinButton,
    pub model_lifecycle_controls: Vec<ModelLifecycleControl>,
}

pub fn build() -> SettingsPane {
    let content = GtkBox::new(Orientation::Vertical, 8);
    content.set_halign(Align::Fill);
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.set_margin_top(2);
    content.set_margin_bottom(2);
    content.set_margin_start(2);
    content.set_margin_end(2);

    let split = GtkBox::new(Orientation::Horizontal, 10);
    split.set_hexpand(true);
    split.set_vexpand(true);

    let (recording_column, silence_timeout_seconds) = build_recording_column();
    let (models_column, model_lifecycle_controls) = build_models_column();
    let divider = Separator::new(Orientation::Vertical);
    divider.set_valign(Align::Fill);
    divider.set_vexpand(true);

    split.append(&recording_column);
    split.append(&divider);
    split.append(&models_column);

    content.append(&split);

    let container = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&content)
        .build();

    SettingsPane {
        container,
        silence_timeout_seconds,
        model_lifecycle_controls,
    }
}

pub fn render(pane: &SettingsPane, silence_timeout_seconds: u64) {
    let bounded = silence_timeout_seconds.min(MAX_TIMEOUT_SECONDS as u64) as f64;
    let current = pane.silence_timeout_seconds.value();
    if (current - bounded).abs() > 0.49 {
        pane.silence_timeout_seconds.set_value(bounded);
    }
}

fn build_recording_column() -> (GtkBox, SpinButton) {
    let column = GtkBox::new(Orientation::Vertical, 10);
    column.set_halign(Align::Fill);
    column.set_hexpand(true);
    column.set_valign(Align::Start);

    let title = Label::new(Some("Recording"));
    title.set_xalign(0.0);
    title.add_css_class("dim-label");

    let timeout_row = GtkBox::new(Orientation::Horizontal, 8);
    timeout_row.set_halign(Align::Start);

    let timeout_label = Label::new(Some("Silence timeout (s)"));
    timeout_label.set_xalign(0.0);

    let adjustment = Adjustment::new(
        10.0,
        MIN_TIMEOUT_SECONDS,
        MAX_TIMEOUT_SECONDS,
        1.0,
        5.0,
        0.0,
    );
    let silence_timeout_seconds = SpinButton::new(Some(&adjustment), 1.0, 0);
    silence_timeout_seconds.set_numeric(true);
    silence_timeout_seconds.set_wrap(false);
    silence_timeout_seconds.set_width_chars(4);
    silence_timeout_seconds.set_tooltip_text(Some("0 disables silence auto-stop"));

    let timeout_hint = Label::new(Some("0 = off"));
    timeout_hint.set_xalign(0.0);
    timeout_hint.add_css_class("dim-label");

    timeout_row.append(&timeout_label);
    timeout_row.append(&silence_timeout_seconds);
    timeout_row.append(&timeout_hint);

    let hint = Label::new(Some("Set silence threshold by clicking the IN meter."));
    hint.set_xalign(0.0);
    hint.set_wrap(true);
    hint.add_css_class("dim-label");

    column.append(&title);
    column.append(&timeout_row);
    column.append(&hint);

    (column, silence_timeout_seconds)
}

fn build_models_column() -> (GtkBox, Vec<ModelLifecycleControl>) {
    let column = GtkBox::new(Orientation::Vertical, 10);
    column.set_halign(Align::Fill);
    column.set_hexpand(true);
    column.set_valign(Align::Start);

    let title = Label::new(Some("Models"));
    title.set_xalign(0.0);
    title.add_css_class("dim-label");

    column.append(&title);

    let mut controls = Vec::with_capacity(ManagedModel::ALL.len());
    for (index, model) in ManagedModel::ALL.iter().enumerate() {
        let subtitle = format!("{}  |  {:.2} GB", model.vendor(), model.size_gb());
        let item = model_lifecycle_item::build(model.title(), &subtitle, model.id());
        item.set_state(InstallState::NotDownloaded);
        column.append(&item.button);
        if index + 1 < ManagedModel::ALL.len() {
            column.append(&Separator::new(Orientation::Horizontal));
        }

        controls.push(ModelLifecycleControl {
            model: *model,
            item,
        });
    }

    (column, controls)
}
