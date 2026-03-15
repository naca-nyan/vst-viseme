pub fn rms(samples: &[f32]) -> f32 {
    let len = samples.len() as f32;
    let mean_square = samples.iter().fold(0.0, |acc, x| acc + x * x / len);
    mean_square.sqrt()
}
