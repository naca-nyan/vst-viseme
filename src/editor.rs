use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use egui_autocomplete::AutoCompleteTextEdit;
use nih_plug::prelude::*;
use nih_plug_egui::egui::*;
use nih_plug_egui::{create_egui_editor, resizable_window::ResizableWindow, widgets, EguiState};
use rosc::OscType;

use crate::{osc, utils::note_friendly_name, VstViseme, VstVisemeParams};

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

struct ParamNameTextEdit<'a> {
    text_field: &'a mut String,
    autocomplete: &'a HashMap<String, OscType>,
    filter_type: &'a ParamType,
}

impl<'a> ParamNameTextEdit<'a> {
    fn new(
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

struct ParamMap<'a> {
    id_salt: &'a str,
    entries: &'a mut Vec<ParamEntry>,
    autocomplete: &'a HashMap<String, OscType>,
    trigger_formatter: fn(&u8) -> String,
    selectable_types: Vec<ParamType>,
    reverse_trigger: bool,
    new_entry: ParamEntry,
}

impl<'a> ParamMap<'a> {
    fn new(
        id_salt: &'a str,
        entries: &'a mut Vec<ParamEntry>,
        autocomplete: &'a HashMap<String, OscType>,
    ) -> Self {
        Self {
            id_salt,
            entries,
            autocomplete,
            trigger_formatter: |v| v.to_string(),
            selectable_types: (0..PARAM_TYPES.len()).collect(),
            reverse_trigger: false,
            new_entry: (0, 0, "".into()),
        }
    }
    fn trigger_formatter(self, trigger_formatter: fn(&u8) -> String) -> Self {
        Self {
            trigger_formatter,
            ..self
        }
    }
    fn selectable_types(self, selectable_types: Vec<usize>) -> Self {
        Self {
            selectable_types,
            ..self
        }
    }
    fn reverse_trigger(self, reverse_trigger: bool) -> Self {
        Self {
            reverse_trigger,
            ..self
        }
    }
    fn new_entry(self, new_entry: ParamEntry) -> Self {
        Self { new_entry, ..self }
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
                        if ui.button("x").clicked() {
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

pub type EditorState = EguiState;
pub fn new_state() -> Arc<EditorState> {
    EguiState::from_size(350, 500)
}

struct UserState {
    receiver: osc::Receiver,
}

pub fn create_editor(
    params: Arc<VstVisemeParams>,
    _async_executor: AsyncExecutor<VstViseme>,
) -> Option<Box<dyn Editor>> {
    let receiver = osc::Receiver::new();
    let egui_state = params.editor_state.clone();
    create_egui_editor(
        egui_state,
        UserState { receiver },
        build,
        move |ctx, setter, state| {
            ResizableWindow::new("res-wind")
                .min_size(Vec2::new(300.0, 280.0))
                .show(ctx, params.editor_state.clone().as_ref(), |ui| {
                    Frame::new()
                        .inner_margin(6.0)
                        .show(ui, |ui| contents(ui, params.clone(), setter, state));
                });
        },
    )
}

fn build(ctx: &Context, _: &mut UserState) {
    let font_candidates = [("Meiryo", "C:/Windows/Fonts/Meiryo.ttc")];
    let mut font_definitions = FontDefinitions::default();
    for (font_name, font_path) in font_candidates {
        if let Ok(font) = std::fs::read(font_path) {
            font_definitions
                .font_data
                .insert(font_name.to_owned(), Arc::new(FontData::from_owned(font)));
            font_definitions
                .families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .insert(0, font_name.to_owned());
        }
    }
    ctx.set_fonts(font_definitions);
}

fn contents(
    ui: &mut Ui,
    params: Arc<VstVisemeParams>,
    setter: &ParamSetter<'_>,
    state: &mut UserState,
) {
    let receiver = &mut state.receiver;
    let receiver_state = receiver.state().read().unwrap().clone();
    ui.heading("Audio");
    Grid::new("audio grid").num_columns(2).show(ui, |ui| {
        ui.label("Gain");
        ui.add(widgets::ParamSlider::for_param(&params.gain, setter));
        ui.end_row();

        ui.label("Address");
        {
            let mut address = params.audio_addr.write().unwrap();
            ui.add(ParamNameTextEdit::new(&mut address, &receiver_state, &2));
        }
        ui.end_row();
    });
    ui.add_space(10.0);
    ui.heading("Midi");
    let mut midi_addrs = params.midi_addrs.write().unwrap();
    let midi_param_map = ParamMap::new("Midi", &mut midi_addrs, &receiver_state)
        .reverse_trigger(true)
        .trigger_formatter(note_friendly_name)
        .new_entry((60, 0, "Item1".into()));
    ui.add(midi_param_map);

    ui.add_space(10.0);
    ui.heading("CC");
    let mut cc_addrs = params.cc_addrs.write().unwrap();
    let cc_param_map = ParamMap::new("CC", &mut cc_addrs, &receiver_state)
        .trigger_formatter(|cc| format!("CC {cc}"))
        .selectable_types(vec![1, 2])
        .new_entry((1, 2, "Float1".into()));
    ui.add(cc_param_map);

    ui.add_space(10.0);
    ui.heading("Monitor");
    if receiver.is_running() {
        if ui.button("Stop monitor").clicked() {
            receiver.stop()
        }
        Grid::new("state grid").num_columns(2).show(ui, |ui| {
            for (k, v) in receiver_state.iter() {
                ui.label(k);
                ui.label(v.to_string());
                ui.end_row();
            }
        });
    } else {
        if ui.button("Start monitor").clicked() {
            const PORT: u16 = 9001;
            receiver
                .init(PORT)
                .unwrap_or_else(|e| nih_error!("Failed to init receiver: {}", e));
        }
    }
}
