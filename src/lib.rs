mod audio;
mod editor;
mod osc;
mod utils;

use std::sync::{Arc, RwLock};

use nih_plug::prelude::*;

use crate::{
    audio::AudioState,
    editor::{EditorState, ParamEntry},
};

pub struct VstViseme {
    params: Arc<VstVisemeParams>,
    sender: osc::Sender,
    audio_state: AudioState,
}

#[derive(Params)]
struct VstVisemeParams {
    #[persist = "editor-state"]
    editor_state: Arc<EditorState>,

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
            audio_state: AudioState::default(),
        }
    }
}

impl Default for VstVisemeParams {
    fn default() -> Self {
        Self {
            editor_state: editor::new_state(),
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

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        editor::create_editor(params, async_executor)
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        const PORT: u16 = 9000;
        self.sender
            .init(PORT)
            .inspect_err(|e| nih_error!("Failed to init sender: {}", e))
            .is_ok()
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
