mod audio;
mod osc;
mod utils;
mod widget;

use std::sync::{Arc, RwLock};

use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{FontData, FontDefinitions, FontFamily, Grid, Vec2},
    resizable_window::ResizableWindow,
    widgets, EguiState,
};

use crate::{
    audio::AudioState,
    utils::note_friendly_name,
    widget::{ParamEntry, ParamNameTextbox},
};

pub struct VstViseme {
    params: Arc<VstVisemeParams>,
    sender: osc::Sender,
    receiver: Arc<osc::Receiver>,
    audio_state: AudioState,
}

#[derive(Params)]
struct VstVisemeParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "bypass"]
    pub bypass: BoolParam,
    #[id = "gain"]
    pub gain: FloatParam,
    #[persist = "audio-address"]
    pub audio_addr: RwLock<String>,
    #[persist = "midi-addresses"]
    pub midi_addrs: RwLock<Vec<ParamEntry>>,
    #[persist = "cc-addresses"]
    pub cc_addrs: RwLock<Vec<ParamEntry>>,
}

impl Default for VstViseme {
    fn default() -> Self {
        Self {
            params: Arc::new(VstVisemeParams::default()),
            sender: osc::Sender::new(),
            receiver: Arc::new(osc::Receiver::new()),
            audio_state: AudioState::default(),
        }
    }
}

impl Default for VstVisemeParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(300, 280),
            bypass: BoolParam::new("Bypass", false).make_bypass(),
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            audio_addr: RwLock::new("Viseme1".into()),
            midi_addrs: RwLock::new(vec![(60, 0, "Item1".into())]),
            cc_addrs: RwLock::new(vec![(1, 2, "Float1".into())]),
        }
    }
}

impl Plugin for VstViseme {
    const NAME: &'static str = "Vst Viseme";
    const VENDOR: &'static str = "Naca Nyan";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "naca.nyan@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let receiver = self.receiver.clone();
        let receiver_state = receiver.state();
        let egui_state = params.editor_state.clone();
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |ctx, _| {
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
            },
            move |egui_ctx, setter, _state| {
                ResizableWindow::new("res-wind")
                    .min_size(Vec2::new(300.0, 280.0))
                    .show(egui_ctx, egui_state.as_ref(), |ui| {
                        let autocomplete = receiver_state.read().unwrap().clone();
                        ui.heading("Audio");
                        Grid::new("audio grid").min_col_width(100.0).show(ui, |ui| {
                            ui.label("Gain");
                            ui.add(widgets::ParamSlider::for_param(&params.gain, setter));
                            ui.end_row();

                            ui.label("Address");
                            {
                                let mut address = params.audio_addr.write().unwrap();
                                ui.add(ParamNameTextbox::new(&mut address, &autocomplete, &[2]));
                            }
                            ui.end_row();
                        });
                        ui.add_space(10.0);
                        ui.heading("Midi");
                        let mut midi_addrs = params.midi_addrs.write().unwrap();
                        let midi_param_map =
                            widget::ParamMap::new("Midi", &mut midi_addrs, &autocomplete)
                                .trigger_formatter(note_friendly_name)
                                .new_entry((60, 0, "Item1".into()));
                        ui.add(midi_param_map);

                        ui.add_space(10.0);
                        ui.heading("CC");
                        let mut cc_addrs = params.cc_addrs.write().unwrap();
                        let cc_param_map =
                            widget::ParamMap::new("CC", &mut cc_addrs, &autocomplete)
                                .trigger_formatter(|cc| format!("CC {cc}"))
                                .available_types((1..3).collect())
                                .new_entry((1, 2, "Float1".into()));
                        ui.add(cc_param_map);

                        ui.add_space(10.0);
                        ui.heading("Monitor");
                        if receiver.is_running() {
                            let state = receiver_state.read().unwrap();
                            Grid::new("state grid").show(ui, |ui| {
                                for (k, v) in state.iter() {
                                    ui.label(k);
                                    ui.label(v.to_string());
                                    ui.end_row();
                                }
                            });
                            if ui.button("Stop monitor").clicked() {
                                receiver.stop()
                            }
                        } else {
                            if ui.button("Start monitor").clicked() {
                                const RECEIVE_PORT: i32 = 9001;
                                receiver.init(RECEIVE_PORT);
                            }
                        }
                    });
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        const PORT: i32 = 9000;
        self.sender.init(PORT)
    }

    fn reset(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if self.params.bypass.value() {
            return ProcessStatus::Normal;
        }

        // process audio
        let gain = self.params.gain.smoothed.next();
        self.audio_state.process(buffer, gain);
        if let Some(rms) = self.audio_state.try_get_rms() {
            let addr = self.params.audio_addr.read().unwrap();
            if !addr.is_empty() {
                self.sender.send(osc::new_float_message(&addr, rms));
            }
            self.audio_state.reset();
        }

        // process midi
        let midi_addrs = self.params.midi_addrs.read().unwrap();
        let cc_addrs = self.params.cc_addrs.read().unwrap();
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn { note, velocity, .. } => {
                    for (_, param_type, name) in midi_addrs.iter().filter(|v| v.0 == note) {
                        if !name.is_empty() {
                            self.sender
                                .send(osc::new_note_on_message(name, param_type, velocity));
                        }
                    }
                }
                NoteEvent::NoteOff { note, .. } => {
                    for (_, param_type, name) in midi_addrs.iter().filter(|v| v.0 == note) {
                        if !name.is_empty() {
                            self.sender
                                .send(osc::new_note_off_message(name, param_type));
                        }
                    }
                }
                NoteEvent::MidiCC { cc, value, .. } => {
                    for (_, param_type, name) in cc_addrs.iter().filter(|v| v.0 == cc) {
                        if !name.is_empty() {
                            self.sender
                                .send(osc::new_cc_message(name, param_type, value));
                        }
                    }
                }
                _ => (),
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for VstViseme {
    const CLAP_ID: &'static str = "com.naca-nyan.vst-viseme";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Sends viseme info to OSC");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Utility, ClapFeature::Stereo];
}

impl Vst3Plugin for VstViseme {
    const VST3_CLASS_ID: [u8; 16] = *b"NacaVstViseme!!!";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(VstViseme);
nih_export_vst3!(VstViseme);
