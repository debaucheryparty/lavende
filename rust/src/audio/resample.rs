pub mod sinc {
use std::collections::VecDeque;
use crate::audio::buffer::PooledBuffer;
pub struct SincResampler {
    ratio: f32,
    index: f32,
    channels: usize,
    taps: usize,
    table: Vec<f32>,
    buffer: Vec<VecDeque<f32>>,
}
impl SincResampler {
    pub fn new(source_rate: u32, target_rate: u32, channels: usize) -> Self {
        let taps = 32;
        let mut table = Vec::with_capacity(taps);
        let m = taps as f32 - 1.0;
        let half_taps = (taps / 2) as f32;
        for i in 0..taps {
            let offset = i as f32 - half_taps;
            let a0 = 0.42;
            let a1 = 0.5;
            let a2 = 0.08;
            let pi_n_m = 2.0 * std::f32::consts::PI * i as f32 / m;
            let window = a0 - a1 * pi_n_m.cos() + a2 * (2.0 * pi_n_m).cos();
            table.push(Self::sinc(offset) * window);
        }
        Self {
            ratio: source_rate as f32 / target_rate as f32,
            index: 0.0,
            channels,
            taps,
            table,
            buffer: vec![VecDeque::from(vec![0.0; taps]); channels],
        }
    }
    fn sinc(x: f32) -> f32 {
        if x.abs() < 1e-6 {
            return 1.0;
        }
        let pi_x = std::f32::consts::PI * x;
        pi_x.sin() / pi_x
    }
    pub fn process(&mut self, input: &[i16], output: &mut PooledBuffer) {
        let num_frames = input.len() / self.channels;
        for frame in 0..num_frames {
            for ch in 0..self.channels {
                self.buffer[ch].pop_front();
                self.buffer[ch].push_back(input[frame * self.channels + ch] as f32);
            }
            while self.index < 1.0 {
                for ch in 0..self.channels {
                    let mut sum = 0.0;
                    for i in 0..self.taps {
                        sum += self.buffer[ch][i] * self.table[i];
                    }
                    output.push(sum.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
                }
                self.index += self.ratio;
            }
            self.index -= 1.0;
        }
    }
    pub fn reset(&mut self) {
        self.index = 0.0;
        for ch in &mut self.buffer {
            for x in ch {
                *x = 0.0;
            }
        }
    }
    pub fn is_passthrough(&self) -> bool {
        (self.ratio - 1.0).abs() < f32::EPSILON
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sinc_function() {
        assert!((SincResampler::sinc(0.0) - 1.0).abs() < 1e-6);
        assert!((SincResampler::sinc(1e-7) - 1.0).abs() < 1e-6);
        let val = SincResampler::sinc(1.0);
        assert!(val.abs() < 0.01); 
        assert!((SincResampler::sinc(2.0) - SincResampler::sinc(-2.0)).abs() < 1e-6);
    }
    #[test]
    fn test_resampler_new_same_rate() {
        let resampler = SincResampler::new(48000, 48000, 2);
        assert!(resampler.is_passthrough());
        assert_eq!(resampler.channels, 2);
        assert_eq!(resampler.taps, 32);
        assert_eq!(resampler.table.len(), 32);
        assert_eq!(resampler.buffer.len(), 2);
    }
    #[test]
    fn test_resampler_new_downsample() {
        let resampler = SincResampler::new(48000, 44100, 2);
        assert!(!resampler.is_passthrough());
        assert!(resampler.ratio > 1.0);
        assert_eq!(resampler.channels, 2);
    }
    #[test]
    fn test_resampler_new_upsample() {
        let resampler = SincResampler::new(44100, 48000, 2);
        assert!(!resampler.is_passthrough());
        assert!(resampler.ratio < 1.0);
    }
    #[test]
    fn test_resampler_reset() {
        let mut resampler = SincResampler::new(48000, 44100, 2);
        resampler.index = 0.5;
        for ch in &mut resampler.buffer {
            for x in ch.iter_mut() {
                *x = 100.0;
            }
        }
        resampler.reset();
        assert_eq!(resampler.index, 0.0);
        for ch in &resampler.buffer {
            for &x in ch.iter() {
                assert_eq!(x, 0.0);
            }
        }
    }
    #[test]
    fn test_resampler_process_empty() {
        let mut resampler = SincResampler::new(48000, 48000, 2);
        let input: Vec<i16> = vec![];
        let mut output = Vec::new();
        resampler.process(&input, &mut output);
        assert!(output.is_empty());
    }
    #[test]
    fn test_resampler_process_silence() {
        let mut resampler = SincResampler::new(48000, 48000, 2);
        let input = vec![0i16; 20]; 
        let mut output = Vec::new();
        resampler.process(&input, &mut output);
        assert!(!output.is_empty());
        for &sample in &output {
            assert_eq!(sample, 0);
        }
    }
    #[test]
    fn test_resampler_process_mono() {
        let mut resampler = SincResampler::new(48000, 48000, 1);
        let input = vec![1000i16; 10]; 
        let mut output = Vec::new();
        resampler.process(&input, &mut output);
        assert!(!output.is_empty());
    }
    #[test]
    fn test_resampler_process_clamp() {
        let mut resampler = SincResampler::new(48000, 48000, 1);
        let input = vec![i16::MAX; 100];
        let mut output = Vec::new();
        resampler.process(&input, &mut output);
        for &sample in &output {
            assert!(sample >= i16::MIN && sample <= i16::MAX);
        }
    }
    #[test]
    fn test_is_passthrough_exact() {
        let resampler = SincResampler::new(48000, 48000, 2);
        assert!(resampler.is_passthrough());
    }
    #[test]
    fn test_is_not_passthrough() {
        let resampler = SincResampler::new(48000, 44100, 2);
        assert!(!resampler.is_passthrough());
    }
    #[test]
    fn test_resampler_table_generation() {
        let resampler = SincResampler::new(48000, 44100, 2);
        assert_eq!(resampler.table.len(), 32);
        for &val in &resampler.table {
            assert!(val.is_finite());
        }
    }
    #[test]
    fn test_resampler_multiple_channels() {
        for channels in 1..=8 {
            let resampler = SincResampler::new(48000, 44100, channels);
            assert_eq!(resampler.buffer.len(), channels);
            for ch_buffer in &resampler.buffer {
                assert_eq!(ch_buffer.len(), 32);
            }
        }
    }
}
}
pub mod linear {
use crate::audio::buffer::PooledBuffer;
pub struct LinearResampler {
    ratio: f32,
    index: f32,
    last_samples: Vec<i16>,
    channels: usize,
}
impl LinearResampler {
    pub fn new(source_rate: u32, target_rate: u32, channels: usize) -> Self {
        Self {
            ratio: source_rate as f32 / target_rate as f32,
            index: 0.0,
            last_samples: vec![0; channels],
            channels,
        }
    }
    pub fn process(&mut self, input: &[i16], output: &mut PooledBuffer) {
        let num_frames = input.len() / self.channels;
        while self.index < num_frames as f32 {
            let idx = self.index as usize;
            let fract = self.index.fract();
            for c in 0..self.channels {
                let s1 = if idx == 0 {
                    self.last_samples[c]
                } else {
                    input[(idx - 1) * self.channels + c]
                } as f32;
                let s2 = if idx < num_frames {
                    input[idx * self.channels + c]
                } else {
                    input[(num_frames - 1) * self.channels + c]
                } as f32;
                output.push((s1 * (1.0 - fract) + s2 * fract) as i16);
            }
            self.index += self.ratio;
        }
        self.index -= num_frames as f32;
        if num_frames > 0 {
            for c in 0..self.channels {
                self.last_samples[c] = input[(num_frames - 1) * self.channels + c];
            }
        }
    }
    pub fn reset(&mut self) {
        self.index = 0.0;
        self.last_samples.fill(0);
    }
    pub fn is_passthrough(&self) -> bool {
        (self.ratio - 1.0).abs() < f32::EPSILON
    }
}
}
pub mod hermite {
use crate::audio::buffer::PooledBuffer;
pub struct HermiteResampler {
    ratio: f32,
    index: f32,
    channels: usize,
    last_samples: Vec<i16>,
}
impl HermiteResampler {
    pub fn new(source_rate: u32, target_rate: u32, channels: usize) -> Self {
        Self {
            ratio: source_rate as f32 / target_rate as f32,
            index: 0.0,
            channels,
            last_samples: vec![0; channels],
        }
    }
    #[inline]
    fn hermite(p: [f32; 4], t: f32) -> f32 {
        let c0 = p[1];
        let c1 = 0.5 * (p[2] - p[0]);
        let c2 = p[0] - 2.5 * p[1] + 2.0 * p[2] - 0.5 * p[3];
        let c3 = 0.5 * (p[3] - p[0]) + 1.5 * (p[1] - p[2]);
        ((c3 * t + c2) * t + c1) * t + c0
    }
    pub fn process(&mut self, input: &[i16], output: &mut PooledBuffer) {
        let num_frames = input.len() / self.channels;
        let num_frames_f = num_frames as f32;
        while self.index < num_frames_f {
            let idx = self.index as usize;
            let t = self.index.fract();
            for ch in 0..self.channels {
                let base_idx = idx * self.channels + ch;
                let p0 = if idx == 0 {
                    self.last_samples[ch]
                } else {
                    input[base_idx - self.channels]
                } as f32;
                let p1 = input[base_idx] as f32;
                let p2 = if idx + 1 < num_frames {
                    input[base_idx + self.channels]
                } else {
                    input[(num_frames - 1) * self.channels + ch]
                } as f32;
                let p3 = if idx + 2 < num_frames {
                    input[base_idx + 2 * self.channels]
                } else {
                    input[(num_frames - 1) * self.channels + ch]
                } as f32;
                let s = Self::hermite([p0, p1, p2, p3], t).clamp(i16::MIN as f32, i16::MAX as f32)
                    as i16;
                output.push(s);
            }
            self.index += self.ratio;
        }
        self.index -= num_frames as f32;
        if num_frames > 0 {
            for ch in 0..self.channels {
                self.last_samples[ch] = input[(num_frames - 1) * self.channels + ch];
            }
        }
    }
    pub fn reset(&mut self) {
        self.index = 0.0;
        self.last_samples.fill(0);
    }
    pub fn is_passthrough(&self) -> bool {
        (self.ratio - 1.0).abs() < f32::EPSILON
    }
}
}
pub use hermite::HermiteResampler;
pub use linear::LinearResampler;
pub use sinc::SincResampler;
use crate::audio::buffer::PooledBuffer;
pub enum Resampler {
    Linear(LinearResampler),
    Hermite(HermiteResampler),
    Sinc(SincResampler),
}
impl Resampler {
    pub fn hermite(source_rate: u32, target_rate: u32, channels: usize) -> Self {
        Self::Hermite(HermiteResampler::new(source_rate, target_rate, channels))
    }
    pub fn linear(source_rate: u32, target_rate: u32, channels: usize) -> Self {
        Self::Linear(LinearResampler::new(source_rate, target_rate, channels))
    }
    pub fn sinc(source_rate: u32, target_rate: u32, channels: usize) -> Self {
        Self::Sinc(SincResampler::new(source_rate, target_rate, channels))
    }
    pub fn is_passthrough(&self) -> bool {
        match self {
            Self::Linear(r) => r.is_passthrough(),
            Self::Hermite(r) => r.is_passthrough(),
            Self::Sinc(r) => r.is_passthrough(),
        }
    }
    pub fn process(&mut self, input: &[i16], output: &mut PooledBuffer) {
        match self {
            Self::Linear(r) => r.process(input, output),
            Self::Hermite(r) => r.process(input, output),
            Self::Sinc(r) => r.process(input, output),
        }
    }
    pub fn reset(&mut self) {
        match self {
            Self::Linear(r) => r.reset(),
            Self::Hermite(r) => r.reset(),
            Self::Sinc(r) => r.reset(),
        }
    }
}