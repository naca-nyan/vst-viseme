use loqa_voice_dsp::pitch::detect_pitch;

pub fn rms(samples: &[f32]) -> f32 {
    let len = samples.len() as f32;
    let mean_square = samples.iter().fold(0.0, |acc, x| acc + x * x / len);
    mean_square.sqrt()
}

pub const MIN_FREQ: f32 = 100.0;
pub const MAX_FREQ: f32 = 800.0;
const CONFIDENCE_THRESHOLD: f32 = 0.5;
pub fn pitch(samples: &[f32], sample_rate: f32) -> Option<f32> {
    let pitch = detect_pitch(samples, sample_rate as u32, MIN_FREQ, MAX_FREQ);
    pitch
        .ok()
        .and_then(|result| (result.confidence > CONFIDENCE_THRESHOLD).then_some(result.frequency))
}
