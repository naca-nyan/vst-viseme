mod audio;
mod editor;
mod osc;
mod utils;
mod widget;

use std::{
    collections::VecDeque,
    sync::{atomic::Ordering, Arc, RwLock},
};

use nih_plug::prelude::*;

use crate::{editor::EditorState, widget::param_map::ParamEntry};

const BUFFER_SIZE: usize = 1024;

#[derive(Default)]
pub struct Meters {
    pitch: AtomicF32,
}

pub struct VstViseme {
    params: Arc<VstVisemeParams>,
    meters: Arc<Meters>,
    buffer: VecDeque<f32>,
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

    #[persist = "pitch-address"]
    pub pitch_addr: RwLock<String>,
    #[id = "pitch-min"]
    pub pitch_min: FloatParam,
    #[id = "pitch-max"]
    pub pitch_max: FloatParam,

    #[persist = "midi-addresses"]
    pub midi_addrs: RwLock<Vec<ParamEntry>>,
    #[persist = "cc-addresses"]
    pub cc_addrs: RwLock<Vec<ParamEntry>>,
}

impl Default for VstViseme {
    fn default() -> Self {
        Self {
            params: Arc::new(VstVisemeParams::default()),
            meters: Arc::new(Meters::default()),
            buffer: VecDeque::with_capacity(BUFFER_SIZE),
        }
    }
}

impl Default for VstVisemeParams {
    fn default() -> Self {
        let pitch_range = FloatRange::Linear {
            min: audio::MIN_FREQ,
            max: audio::MAX_FREQ,
        };
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
            audio_addr: RwLock::new("Volume1".into()),
            pitch_addr: RwLock::new("Pitch1".into()),
            pitch_min: FloatParam::new("Pitch min", audio::MIN_FREQ, pitch_range),
            pitch_max: FloatParam::new("Pitch max", audio::MAX_FREQ, pitch_range),
            midi_addrs: RwLock::new(vec![(60, 0, "Item1".into())]),
            cc_addrs: RwLock::new(vec![(1, 2, "Float1".into())]),
        }
    }
}

pub enum Task {
    UpdateSampleRate(f32),
    ProcessSamples([f32; BUFFER_SIZE]),
    NoteEvent(NoteEvent<()>),
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
    type BackgroundTask = Task;

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        const PORT: u16 = 9000;
        let mut sender = osc::Sender::new();
        sender
            .init(PORT)
            .unwrap_or_else(|e| nih_error!("Failed to init sender: {}", e));
        let params = self.params.clone();
        let meters = self.meters.clone();
        let sample_rate = Arc::new(AtomicF32::new(48_000.0));
        Box::new(move |task| match task {
            Task::UpdateSampleRate(value) => sample_rate.store(value, Ordering::Relaxed),
            Task::ProcessSamples(samples) => {
                let rms = audio::rms(&samples);
                {
                    let addr = params.audio_addr.read().unwrap();
                    if !addr.is_empty() {
                        sender.send(osc::new_float_message(&addr, rms));
                    }
                }
                const GATE_THRESHOLD: f32 = 0.01;
                if rms > GATE_THRESHOLD {
                    let pitch = audio::pitch(&samples, sample_rate.load(Ordering::Relaxed));
                    if let Some((frequency, confidence)) = pitch {
                        const CONFIDENCE_MIN: f32 = 0.5;
                        if confidence > CONFIDENCE_MIN {
                            meters.pitch.store(frequency, Ordering::Relaxed);
                            let addr = params.pitch_addr.read().unwrap();
                            let min = params.pitch_min.value();
                            let max = params.pitch_max.value();
                            let normalized = (frequency - min) / (max - min);
                            if !addr.is_empty() {
                                sender.send(osc::new_float_message(&addr, normalized));
                            }
                        }
                    }
                }
            }
            Task::NoteEvent(event) => match event {
                NoteEvent::NoteOn { note, velocity, .. } => {
                    let midi_addrs = params.midi_addrs.read().unwrap();
                    for (_, param_type, name) in midi_addrs.iter().filter(|v| v.0 == note) {
                        if !name.is_empty() {
                            sender.send(osc::new_note_on_message(name, param_type, velocity));
                        }
                    }
                }
                NoteEvent::NoteOff { note, .. } => {
                    let midi_addrs = params.midi_addrs.read().unwrap();
                    for (_, param_type, name) in midi_addrs.iter().filter(|v| v.0 == note) {
                        if !name.is_empty() {
                            sender.send(osc::new_note_off_message(name, param_type));
                        }
                    }
                }
                NoteEvent::MidiCC { cc, value, .. } => {
                    let cc_addrs = params.cc_addrs.read().unwrap();
                    for (_, param_type, name) in cc_addrs.iter().filter(|v| v.0 == cc) {
                        if !name.is_empty() {
                            sender.send(osc::new_cc_message(name, param_type, value));
                        }
                    }
                }
                _ => (),
            },
        })
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let meters = self.meters.clone();
        editor::create_editor(params, meters, async_executor)
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        context.execute(Task::UpdateSampleRate(buffer_config.sample_rate));
        true
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
        for channel_samples in buffer.iter_samples() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();
            let channels = channel_samples.len();
            let mut sum = 0.0;
            for sample in channel_samples {
                sum += *sample * gain;
            }
            let mean = sum / channels as f32;
            self.buffer.push_back(mean);
        }
        if self.buffer.len() >= BUFFER_SIZE {
            let mut samples = [0.0; BUFFER_SIZE];
            for (dst, src) in samples.iter_mut().zip(self.buffer.drain(..BUFFER_SIZE)) {
                *dst = src;
            }
            context.execute_background(Task::ProcessSamples(samples));
        }

        // process midi
        while let Some(event) = context.next_event() {
            context.execute_background(Task::NoteEvent(event));
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
