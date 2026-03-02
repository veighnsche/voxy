use gtk4::{prelude::*, CellRendererText, ComboBox, ListStore};
use voxy_core::TranscriptionModelId;

const COL_ID: u32 = 0;
const COL_LABEL: u32 = 1;

pub fn build() -> ComboBox {
    let rows = ListStore::new(&[String::static_type(), String::static_type()]);

    for model in TranscriptionModelId::ALL {
        rows.insert_with_values(
            None,
            &[(COL_ID, &model.as_api_id()), (COL_LABEL, &model.as_label())],
        );
    }

    let dropdown = ComboBox::with_model(&rows);
    dropdown.set_id_column(COL_ID as i32);

    let renderer = CellRendererText::new();
    dropdown.pack_start(&renderer, true);
    dropdown.add_attribute(&renderer, "text", COL_LABEL as i32);

    dropdown.set_active_id(Some(TranscriptionModelId::default().as_api_id()));
    dropdown.set_tooltip_text(Some("Transcription model"));
    dropdown.set_size_request(140, -1);
    dropdown
}
