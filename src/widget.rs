use std::collections::{BTreeSet, HashMap};

use egui_autocomplete::AutoCompleteTextEdit;
use nih_plug_egui::egui::{Align, ComboBox, Grid, Layout, Response, Ui, Widget};
use rosc::OscType;

type Trigger = u8;
type ParamType = usize;
pub type ParamEntry = (Trigger, ParamType, String);
const PARAM_TYPES: &[&str] = &["Bool", "Int", "Float"];

fn param_type_from_osc(t: &OscType) -> ParamType {
    match t {
        OscType::Bool(_) => 0,
        OscType::Int(_) => 1,
        OscType::Float(_) => 2,
        _ => 3,
    }
}

pub struct ParamNameTextbox<'a> {
    text_field: &'a mut String,
    autocomplete: &'a HashMap<String, OscType>,
    available_types: &'a [ParamType],
}

impl<'a> ParamNameTextbox<'a> {
    pub fn new(
        text_field: &'a mut String,
        autocomplete: &'a HashMap<String, OscType>,
        available_types: &'a [ParamType],
    ) -> Self {
        Self {
            text_field,
            autocomplete,
            available_types,
        }
    }
}

impl Widget for ParamNameTextbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let text_field = self.text_field;
        let search = &self
            .autocomplete
            .iter()
            .filter(|(_, t)| self.available_types.contains(&param_type_from_osc(t)))
            .map(|(s, _)| s)
            .collect::<BTreeSet<_>>();
        ui.add(
            AutoCompleteTextEdit::new(text_field, search)
                .popup_on_focus(true)
                .set_text_edit_properties(|text_edit| text_edit.desired_width(f32::INFINITY)),
        )
    }
}

pub struct ParamMap<'a> {
    id_salt: &'a str,
    entries: &'a mut Vec<ParamEntry>,
    autocomplete: &'a HashMap<String, OscType>,
    trigger_formatter: fn(&u8) -> String,
    available_types: Vec<ParamType>,
    reverse_trigger: bool,
    new_entry: ParamEntry,
}

impl<'a> ParamMap<'a> {
    pub fn new(
        id_salt: &'a str,
        entries: &'a mut Vec<ParamEntry>,
        autocomplete: &'a HashMap<String, OscType>,
    ) -> Self {
        Self {
            id_salt,
            entries,
            autocomplete,
            trigger_formatter: |v| v.to_string(),
            available_types: (0..PARAM_TYPES.len()).collect(),
            reverse_trigger: false,
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
    pub fn reverse_trigger(self, reverse_trigger: bool) -> Self {
        Self {
            reverse_trigger,
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
        let entries = self.entries;
        let mut delete = None;
        let grid = Grid::new(format!("{id_salt} grid"))
            .num_columns(2)
            .striped(true);
        grid.show(ui, |ui| {
            for (i, (trigger, param_type, name)) in entries.iter_mut().enumerate() {
                ComboBox::from_id_salt(format!("{id_salt} key {i} combobox"))
                    .width(50.0)
                    .selected_text(formatter(trigger))
                    .show_ui(ui, |ui| {
                        if self.reverse_trigger {
                            for n in (0..128).rev() {
                                ui.selectable_value(trigger, n, formatter(&n));
                            }
                        } else {
                            for n in 0..128 {
                                ui.selectable_value(trigger, n, formatter(&n));
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("x").clicked() {
                            delete = Some(i);
                        }
                        ComboBox::from_id_salt(format!("{id_salt} param {i} combobox"))
                            .width(56.0)
                            .selected_text(PARAM_TYPES[*param_type])
                            .show_ui(ui, |ui| {
                                for &t in available_types {
                                    ui.selectable_value(param_type, t, PARAM_TYPES[t]);
                                }
                            });
                        ui.add(ParamNameTextbox::new(
                            name,
                            self.autocomplete,
                            &self.available_types,
                        ));
                    });
                });
                ui.end_row();
            }
        });
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
    }
}
