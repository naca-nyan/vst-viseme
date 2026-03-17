use std::collections::{BTreeSet, HashMap};

use egui_autocomplete::AutoCompleteTextEdit;
use nih_plug_egui::egui::{Response, Ui, Widget};
use rosc::OscType;

type ParamType = usize;

fn param_type_from_osc(t: &OscType) -> ParamType {
    match t {
        OscType::Bool(_) => 0,
        OscType::Int(_) => 1,
        OscType::Float(_) => 2,
        _ => 3,
    }
}

pub struct ParamNameTextEdit<'a> {
    text_field: &'a mut String,
    autocomplete: &'a HashMap<String, OscType>,
    filter_type: &'a ParamType,
}

impl<'a> ParamNameTextEdit<'a> {
    pub fn new(
        text_field: &'a mut String,
        autocomplete: &'a HashMap<String, OscType>,
        filter_type: &'a ParamType,
    ) -> Self {
        Self {
            text_field,
            autocomplete,
            filter_type,
        }
    }
}

impl Widget for ParamNameTextEdit<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let text_field = self.text_field;
        let search = &self
            .autocomplete
            .iter()
            .filter_map(|(s, t)| self.filter_type.eq(&param_type_from_osc(t)).then_some(s))
            .collect::<BTreeSet<_>>();
        ui.add(
            AutoCompleteTextEdit::new(text_field, search)
                .popup_on_focus(true)
                .set_text_edit_properties(|text_edit| text_edit.desired_width(f32::INFINITY)),
        )
    }
}
