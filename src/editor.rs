use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use nih_plug::prelude::*;
use nih_plug_egui::egui::*;
use nih_plug_egui::{create_egui_editor, resizable_window::ResizableWindow, widgets, EguiState};
use serde_json::Value;

use crate::{
    osc,
    utils::note_friendly_name,
    widget::{meter::Meter, param_map::ParamMap, param_name_text_edit::ParamNameTextEdit},
    Meters, VstViseme, VstVisemeParams,
};

pub type EditorState = EguiState;
pub fn new_state() -> Arc<EditorState> {
    EguiState::from_size(350, 400)
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Main,
    Monitor,
    Config,
}

const TABS: [(Tab, &str); 3] = {
    use Tab::*;
    [(Main, "Main"), (Monitor, "Monitor"), (Config, "Config")]
};

type DialogResult = Result<String, String>;

struct UserState {
    receiver: osc::Receiver,
    tab: Tab,
    dialog_result: Arc<Mutex<DialogResult>>, // この Mutex が lock されているときは dialog が開いている
}

pub fn create_editor(
    params: Arc<VstVisemeParams>,
    meters: Arc<Meters>,
    _async_executor: AsyncExecutor<VstViseme>,
) -> Option<Box<dyn Editor>> {
    let receiver = osc::Receiver::new();
    let tab = Tab::Main;
    let egui_state = params.editor_state.clone();
    create_egui_editor(
        egui_state,
        UserState {
            receiver,
            tab,
            dialog_result: Arc::new(Mutex::new(Ok(String::new()))),
        },
        build,
        move |ctx, setter, state| {
            ResizableWindow::new("res-wind")
                .min_size(Vec2::new(300.0, 280.0))
                .show(ctx, params.editor_state.as_ref(), |_ui| {
                    TopBottomPanel::top("top-panel").show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            for &(tab, label) in &TABS {
                                ui.selectable_value(&mut state.tab, tab, label);
                            }
                        });
                    });
                    CentralPanel::default().show(ctx, |ui| match state.tab {
                        Tab::Main => show_main(ui, params.clone(), meters.clone(), setter, state),
                        Tab::Monitor => show_monitor(ui, state),
                        Tab::Config => show_config(ui, params.clone(), state),
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

fn show_main(
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
        ui.add(Meter::new(
            meters.rms.load(Ordering::Relaxed),
            &params.volume_min,
            &params.volume_max,
            setter,
        ));
        ui.end_row();
        ui.label("");
        {
            let mut address = params.volume_addr.write().unwrap();
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
    let midi_param_map = ParamMap::new("Midi", &mut midi_addrs, &receiver_state, &meters.midi)
        .reverse_trigger(true)
        .trigger_formatter(note_friendly_name)
        .new_entry((60, 0, "Item1".into()));
    ui.add(midi_param_map);

    ui.add_space(10.0);
    ui.heading("CC");
    let mut cc_addrs = params.cc_addrs.write().unwrap();
    let cc_param_map = ParamMap::new("CC", &mut cc_addrs, &receiver_state, &meters.cc)
        .trigger_formatter(|cc| format!("CC {cc}"))
        .selectable_types(vec![1, 2])
        .new_entry((1, 2, "Float1".into()));
    ui.add(cc_param_map);
}

fn show_monitor(ui: &mut Ui, state: &mut UserState) {
    let receiver = &mut state.receiver;
    ui.heading("Monitor");
    ui.label("VRChat からパラメータの変化を受信し、パラメータ名の補完候補に追加します。");
    let rec_mark = RichText::color(RichText::new("●"), Color32::RED);
    if receiver.is_running() {
        if ui.selectable_label(true, rec_mark).clicked() {
            receiver.stop();
        }
    } else {
        if ui.button(rec_mark).clicked() {
            const PORT: u16 = 9001;
            receiver
                .init(PORT)
                .unwrap_or_else(|e| nih_error!("Failed to init receiver: {}", e));
        }
    }
    let receiver_state = receiver.state().read().unwrap().clone();
    if receiver.is_running() && receiver_state.is_empty() {
        ui.label("何も受信されていません");
    }
    Grid::new("state grid").striped(true).show(ui, |ui| {
        for (k, v) in receiver_state.iter() {
            ui.label(k);
            ui.label(type_as_str(v));
            if receiver.is_running() {
                ui.label(value_str(v));
            }
            ui.end_row();
        }
    });
}

fn show_config(ui: &mut Ui, params: Arc<VstVisemeParams>, state: &mut UserState) {
    ui.heading("Config");
    let dialog_lock = state.dialog_result.try_lock();
    let dialog_opened = dialog_lock.is_err();
    ui.add_enabled_ui(!dialog_opened, |ui| {
        if ui.button("Import parameters").clicked() {
            let params = params.clone();
            spawn_dialog(&state, move |dialog| {
                let path = dialog.add_filter("json", &["json"]).pick_file();
                if let Some(path) = path {
                    let data =
                        std::fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;
                    let json_value: BTreeMap<String, Value> =
                        serde_json::from_slice(&data).map_err(|e| format!("Invalid json: {e}"))?;
                    let serialized: BTreeMap<String, String> = json_value
                        .into_iter()
                        .map(|(k, v)| (k, v.to_string()))
                        .collect();
                    params.deserialize_fields(&serialized);
                    Ok("Import succeeded!".to_owned())
                } else {
                    Ok("".to_owned())
                }
            });
        }
        if ui.button("Export parameters").clicked() {
            let params = params.clone();
            spawn_dialog(&state, move |dialog| {
                let value_map: BTreeMap<String, Value> = params
                    .serialize_fields()
                    .into_iter()
                    .filter_map(|(k, v)| {
                        if k == "editor-state" {
                            return None;
                        }
                        let value = serde_json::from_str::<Value>(&v).ok()?;
                        Some((k, value))
                    })
                    .collect();
                let json = serde_json::to_string_pretty(&value_map)
                    .map_err(|e| format!("Failed to format json: {e}"))?;
                let path = dialog.set_file_name("vst-viseme-params.json").save_file();
                if let Some(path) = path {
                    std::fs::write(path, &json)
                        .map_err(|e| format!("Failed to write file: {e}"))?;
                    Ok("Export succeeded!".to_owned())
                } else {
                    Ok("".to_owned())
                }
            });
        }
        if let Ok(result) = dialog_lock {
            match result.as_ref() {
                Ok(msg) => ui.label(msg),
                Err(error) => ui.colored_label(Color32::RED, error),
            };
        }
    });
}

fn spawn_dialog(
    state: &UserState,
    task: impl FnOnce(rfd::FileDialog) -> DialogResult + Send + 'static,
) {
    let dialog_result = state.dialog_result.clone();
    std::thread::spawn(move || {
        // Dialog が開いている間 (task が終わるまで) dialog_result を lock し続ける
        let mut error = dialog_result.lock().unwrap_or_else(|e| e.into_inner());
        let dialog = rfd::FileDialog::new();
        *error = task(dialog);
    });
}

fn type_as_str(t: &rosc::OscType) -> &'static str {
    use rosc::OscType::*;
    match t {
        Int(_) => "Int",
        Float(_) => "Float",
        String(_) => "String",
        Bool(_) => "Bool",
        _ => "Unknown",
    }
}

fn value_str(t: &rosc::OscType) -> String {
    use rosc::OscType::*;
    match t {
        Int(v) => v.to_string(),
        Float(v) => v.to_string(),
        String(v) => v.to_string(),
        Bool(v) => v.to_string(),
        _ => "".to_string(),
    }
}
