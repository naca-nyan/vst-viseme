use nih_plug_egui::egui::{ComboBox, Grid, Response, Ui, Widget};

type Trigger = u8;
type ParamType = usize;
pub type ParamEntry = (Trigger, ParamType, String);
const PARAM_TYPES: &[&str] = &["Bool", "Int", "Float"];

pub struct ParamMap<'a> {
    id_salt: &'a str,
    entries: &'a mut Vec<ParamEntry>,
    trigger_formatter: fn(&u8) -> String,
    available_types: Vec<usize>,
    new_entry: ParamEntry,
}

impl<'a> ParamMap<'a> {
    pub fn new(id_salt: &'a str, entries: &'a mut Vec<ParamEntry>) -> Self {
        Self {
            id_salt,
            entries,
            trigger_formatter: |v| v.to_string(),
            available_types: (0..3).collect(),
            new_entry: (0, 0, "".into()),
        }
    }
    pub fn trigger_formatter(self, trigger_formatter: fn(&u8) -> String) -> Self {
        Self {
            trigger_formatter,
            ..self
        }
    }
    pub fn available_types(self, available_types: Vec<usize>) -> Self {
        Self {
            available_types,
            ..self
        }
    }
    pub fn new_entry(self, new_entry: ParamEntry) -> Self {
        Self { new_entry, ..self }
    }
}

impl Widget for ParamMap<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let id_salt = self.id_salt;
        let formatter = self.trigger_formatter;
        let available_types = &self.available_types;
        let col_width = 70.0;
        let midi_grid = Grid::new(format!("{id_salt} grid")).min_col_width(col_width);
        let inner_resp = midi_grid.show(ui, |ui| {
            let entries = self.entries;
            let mut delete = None;
            for (i, (trigger, param_type, name)) in entries.iter_mut().enumerate() {
                ComboBox::from_id_salt(format!("{id_salt} key {i} combobox"))
                    .width(col_width)
                    .selected_text(formatter(trigger))
                    .show_ui(ui, |ui| {
                        for n in (0..127u8).rev() {
                            ui.selectable_value(trigger, n, formatter(&n));
                        }
                    });
                ui.text_edit_singleline(name);
                ComboBox::from_id_salt(format!("{id_salt} param {i} combobox"))
                    .width(col_width)
                    .selected_text(PARAM_TYPES[*param_type])
                    .show_ui(ui, |ui| {
                        for &t in available_types {
                            ui.selectable_value(param_type, t, PARAM_TYPES[t]);
                        }
                    });
                if ui.button("x").clicked() {
                    delete = Some(i);
                }
                ui.end_row();
            }
            if let Some(i) = delete {
                entries.remove(i);
            }
            let response = ui.button("Add");
            if entries.len() < 128 && response.clicked() {
                let mut new_entry = self.new_entry;
                if let Some(max) = entries.iter().map(|v| v.0).max() {
                    new_entry.0 = max + 1;
                }
                entries.push(new_entry);
            }
            response
        });
        inner_resp.response
    }
}
