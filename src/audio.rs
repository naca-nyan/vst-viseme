use nih_plug::prelude::Buffer;

pub struct AudioState {
    /// 蓄積された二乗和（RMS計算用）
    acc_sum_squares: f32,
    /// 蓄積されたサンプル数
    acc_sample_count: usize,
    /// processが呼ばれた回数のカウンター
    process_count: usize,
}

impl Default for AudioState {
    fn default() -> Self {
        AudioState {
            acc_sum_squares: 0.0,
            acc_sample_count: 0,
            process_count: 0,
        }
    }
}

/// 何回の process ごとに rms を計算するか
const RMS_INTERVAL: usize = 4;

impl AudioState {
    pub fn process(&mut self, buffer: &mut Buffer, gain: f32) {
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                let s = *sample * gain;
                self.acc_sum_squares += s * s;
                self.acc_sample_count += 1;
            }
        }
        self.process_count += 1;
    }
    pub fn try_get_rms(&self) -> Option<f32> {
        if self.process_count >= RMS_INTERVAL && self.acc_sample_count > 0 {
            Some((self.acc_sum_squares / self.acc_sample_count as f32).sqrt())
        } else {
            None
        }
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
