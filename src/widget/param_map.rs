use std::{
    collections::HashMap,
    sync::atomic::{AtomicU8, Ordering},
};

use nih_plug_egui::egui::{Align, ComboBox, Grid, Layout, Response, Ui, Widget};
use rosc::OscType;

use crate::widget::param_name_text_edit::ParamNameTextEdit;

type Trigger = u8;
type ParamType = usize;
pub type ParamEntry = (Trigger, ParamType, String);
const PARAM_TYPES: &[&str] = &["Bool", "Int", "Float"];

pub struct ParamMap<'a> {
    id_salt: &'a str,
    entries: &'a mut Vec<ParamEntry>,
    autocomplete: &'a HashMap<String, OscType>,
    meter: &'a AtomicU8,
    trigger_formatter: fn(&u8) -> String,
    selectable_types: Vec<ParamType>,
    reverse_trigger: bool,
    new_trigger: Trigger,
}

impl<'a> ParamMap<'a> {
    pub fn new(
        id_salt: &'a str,
        entries: &'a mut Vec<ParamEntry>,
        autocomplete: &'a HashMap<String, OscType>,
        meter: &'a AtomicU8,
    ) -> Self {
        Self {
            id_salt,
            entries,
            autocomplete,
            meter,
            trigger_formatter: |v| v.to_string(),
            selectable_types: (0..PARAM_TYPES.len()).collect(),
            reverse_trigger: false,
            new_trigger: 0,
        }
    }
    pub fn trigger_formatter(self, trigger_formatter: fn(&u8) -> String) -> Self {
        Self {
            trigger_formatter,
            ..self
        }
    }
    pub fn selectable_types(self, selectable_types: Vec<usize>) -> Self {
        Self {
            selectable_types,
            ..self
        }
    }
    pub fn reverse_trigger(self, reverse_trigger: bool) -> Self {
        Self {
            reverse_trigger,
            ..self
        }
    }
    pub fn new_trigger(self, new_trigger: Trigger) -> Self {
        Self {
            new_trigger,
            ..self
        }
    }
}

impl Widget for ParamMap<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let id_salt = self.id_salt;
        let formatter = self.trigger_formatter;
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
                        if ui.button("×").clicked() {
                            delete = Some(i);
                        }
                        ComboBox::from_id_salt(format!("{id_salt} param {i} combobox"))
                            .width(56.0)
                            .selected_text(PARAM_TYPES[*param_type])
                            .show_ui(ui, |ui| {
                                for &t in &self.selectable_types {
                                    ui.selectable_value(param_type, t, PARAM_TYPES[t]);
                                }
                            });
                        ui.add(ParamNameTextEdit::new(name, self.autocomplete, param_type));
                    });
                });
                ui.end_row();
            }
        });
        if let Some(i) = delete {
            entries.remove(i);
        }
        ui.horizontal(|ui| {
            let meter = self.meter.load(Ordering::Relaxed);
            let response = ui.button("＋");
            if entries.len() < 128 && response.clicked() {
                let new_trigger = if let Some(max) = entries.iter().map(|v| v.0).max() {
                    max + 1
                } else if meter != 0 {
                    meter
                } else {
                    self.new_trigger
                };
                let new_type = entries
                    .last()
                    .map(|v| v.1)
                    .unwrap_or(*self.selectable_types.first().unwrap());
                entries.push((new_trigger, new_type, String::new()));
            };
            if meter != 0 {
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    let resp = ui.label(formatter(&meter));
                    if resp.clicked() {
                        self.meter.store(0, Ordering::Relaxed);
                    }
                });
            }
            response
        })
        .inner
    }
}
