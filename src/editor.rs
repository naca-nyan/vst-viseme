use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_egui::egui::*;
use nih_plug_egui::{create_egui_editor, resizable_window::ResizableWindow, widgets, EguiState};

use crate::{
    osc,
    utils::note_friendly_name,
    widget::{meter::Meter, param_map::ParamMap, param_name_text_edit::ParamNameTextEdit},
    Meters, VstViseme, VstVisemeParams,
};

pub type EditorState = EguiState;
pub fn new_state() -> Arc<EditorState> {
    EguiState::from_size(350, 500)
}

struct UserState {
    receiver: osc::Receiver,
}

pub fn create_editor(
    params: Arc<VstVisemeParams>,
    meters: Arc<Meters>,
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
                    Frame::new().inner_margin(6.0).show(ui, |ui| {
                        contents(ui, params.clone(), meters.clone(), setter, state)
                    });
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
    meters: Arc<Meters>,
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

        ui.label("Volume");
        {
            let mut address = params.audio_addr.write().unwrap();
            ui.add(ParamNameTextEdit::new(&mut address, &receiver_state, &2));
        }
        ui.end_row();

        ui.label("Pitch");
        ui.add(Meter::new(
            meters.pitch.load(Ordering::Relaxed),
            &params.pitch_min,
            &params.pitch_max,
            setter,
        ));
        ui.end_row();
        ui.label("");
        {
            let mut address = params.pitch_addr.write().unwrap();
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
