use gtk4::{prelude::*, Label};

#[derive(Clone)]
pub struct RecordingIndicator {
    pub label: Label,
}

pub fn build() -> RecordingIndicator {
    let label = Label::new(None);
    label.set_xalign(0.0);
    label.set_width_chars(6);
    label.set_markup("<span foreground=\"#4b5563\" weight=\"bold\" size=\"11000\">● REC</span>");
    label.set_visible(true);
    RecordingIndicator { label }
}

pub fn render(indicator: &RecordingIndicator, recording: bool) {
    if recording {
        indicator
            .label
            .set_markup("<span foreground=\"#dc2626\" weight=\"bold\" size=\"11000\">● REC</span>");
    } else {
        indicator
            .label
            .set_markup("<span foreground=\"#4b5563\" weight=\"bold\" size=\"11000\">● REC</span>");
    }
}
