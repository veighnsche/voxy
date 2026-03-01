use gtk4::{glib::object::Cast, pango, prelude::*, CellRendererText, ComboBox, ListStore};
use voxy_stt::TranscriptionModel;

const GROUP_ROW_PREFIX: &str = "__group__";
const COL_ID: u32 = 0;
const COL_LABEL: u32 = 1;
const COL_IS_GROUP: u32 = 2;
const COL_SENSITIVE: u32 = 3;
const FONT_WEIGHT_NORMAL: i32 = 400;
const FONT_WEIGHT_GROUP: i32 = 700;

pub fn build() -> ComboBox {
    let rows = ListStore::new(&[
        String::static_type(),
        String::static_type(),
        bool::static_type(),
        bool::static_type(),
    ]);
    let mut current_adapter_id: Option<&'static str> = None;

    for model in TranscriptionModel::ALL {
        let adapter = model.adapter();
        let adapter_id = adapter.as_id();
        if current_adapter_id != Some(adapter_id) {
            let group_id = group_row_id(adapter_id);
            rows.insert_with_values(
                None,
                &[
                    (COL_ID, &group_id),
                    (COL_LABEL, &adapter.as_label()),
                    (COL_IS_GROUP, &true),
                    (COL_SENSITIVE, &false),
                ],
            );
            current_adapter_id = Some(adapter_id);
        }

        rows.insert_with_values(
            None,
            &[
                (COL_ID, &model.as_api_id()),
                (COL_LABEL, &format!("  {}", model.as_label())),
                (COL_IS_GROUP, &false),
                (COL_SENSITIVE, &true),
            ],
        );
    }

    let dropdown = ComboBox::with_model(&rows);
    dropdown.set_id_column(COL_ID as i32);

    let renderer = CellRendererText::new();
    dropdown.pack_start(&renderer, true);
    dropdown.add_attribute(&renderer, "text", COL_LABEL as i32);
    dropdown.add_attribute(&renderer, "sensitive", COL_SENSITIVE as i32);
    dropdown.set_cell_data_func(&renderer, move |_, cell, model, iter| {
        let is_group: bool = model.get(iter, COL_IS_GROUP as i32);
        let Some(text_cell) = cell.downcast_ref::<CellRendererText>() else {
            return;
        };

        text_cell.set_weight(if is_group {
            FONT_WEIGHT_GROUP
        } else {
            FONT_WEIGHT_NORMAL
        });
        text_cell.set_style(pango::Style::Normal);
        text_cell.set_foreground(if is_group { Some("#6b7280") } else { None });
    });

    dropdown.set_active_id(Some(TranscriptionModel::default().as_api_id()));
    dropdown.set_tooltip_text(Some("Transcription model"));
    dropdown.set_size_request(110, -1);
    dropdown
}

pub fn is_group_row_id(id: &str) -> bool {
    id.starts_with(GROUP_ROW_PREFIX)
}

fn group_row_id(adapter_id: &str) -> String {
    format!("{GROUP_ROW_PREFIX}{adapter_id}")
}
