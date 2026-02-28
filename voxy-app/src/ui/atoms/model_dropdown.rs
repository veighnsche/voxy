use gtk4::{prelude::*, ComboBoxText};
use voxy_stt::TranscriptionModel;

pub fn build() -> ComboBoxText {
    let dropdown = ComboBoxText::new();

    for model in TranscriptionModel::ALL {
        dropdown.append(Some(model.as_api_id()), model.as_label());
    }

    dropdown.set_active_id(Some(TranscriptionModel::default().as_api_id()));
    dropdown.set_tooltip_text(Some("Transcription model"));
    dropdown.set_size_request(110, -1);
    dropdown
}
