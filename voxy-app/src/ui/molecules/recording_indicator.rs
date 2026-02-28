use gtk4::{prelude::*, Label};

#[derive(Clone)]
pub struct RecordingIndicator {
    pub label: Label,
}

pub fn build() -> RecordingIndicator {
    let label = Label::new(Some(""));
    label.set_xalign(0.0);
    label.set_visible(false);
    RecordingIndicator { label }
}

pub fn render(indicator: &RecordingIndicator, recording: bool) {
    if recording {
        indicator
            .label
            .set_markup("<span foreground=\"#dc2626\" weight=\"bold\" size=\"12000\">● REC</span>");
        indicator.label.set_visible(true);
    } else {
        indicator.label.set_text("");
        indicator.label.set_visible(false);
    }
}
