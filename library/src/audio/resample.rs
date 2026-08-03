pub mod sinc {
    use crate::audio::buffer::PooledBuffer;
    pub struct SincResampler {
        ratio: f32,
        index: f32,
        channels: usize,
        taps: usize,
        table: Vec<f32>,
        buffer: Vec<f32>,
        head: usize,
    }
    impl SincResampler {
        pub fn new(source_rate: u32, target_rate: u32, channels: usize) -> Self {
            let taps = 24;
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
                buffer: vec![0.0; channels * taps * 2],
                head: 0,
            }
        }
        #[inline(always)]
        fn sinc(x: f32) -> f32 {
            if x.abs() < 1e-6 {
                return 1.0;
            }
            let pi_x = std::f32::consts::PI * x;
            pi_x.sin() / pi_x
        }
        pub fn process(&mut self, input: &[i16], output: &mut PooledBuffer) {
            let num_frames = input.len() / self.channels;
            let taps = self.taps;
            let est_output = ((num_frames as f32 / self.ratio) as usize + 2) * self.channels;
            output.reserve(est_output);
            for frame in 0..num_frames {
                for ch in 0..self.channels {
                    let val = input[frame * self.channels + ch] as f32;
                    let base = ch * taps * 2;
                    self.buffer[base + self.head] = val;
                    self.buffer[base + self.head + taps] = val;
                }
                self.head += 1;
                if self.head >= taps {
                    self.head = 0;
                }
                while self.index < 1.0 {
                    for ch in 0..self.channels {
                        let base = ch * taps * 2 + self.head;
                        let buf = &self.buffer[base..base + taps];
                        let mut sum = 0.0f32;
                        for i in 0..taps {
                            sum += buf[i] * self.table[i];
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
            self.head = 0;
            self.buffer.fill(0.0);
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
            assert_eq!(resampler.taps, 24);
            assert_eq!(resampler.table.len(), 24);
            assert_eq!(resampler.buffer.len(), 96);
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
            resampler.buffer.fill(100.0);
            resampler.reset();
            assert_eq!(resampler.index, 0.0);
            for &x in &resampler.buffer {
                assert_eq!(x, 0.0);
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
            assert_eq!(resampler.table.len(), 24);
            for &val in &resampler.table {
                assert!(val.is_finite());
            }
        }
        #[test]
        fn test_resampler_multiple_channels() {
            for channels in 1..=8 {
                let resampler = SincResampler::new(48000, 44100, channels);
                assert_eq!(resampler.buffer.len(), channels * 48);
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
            let num_frames_f = num_frames as f32;
            let est_output = ((num_frames_f / self.ratio) as usize + 2) * self.channels;
            output.reserve(est_output);

            while self.index < num_frames_f {
                let idx = self.index as usize;
                let fract = self.index.fract();
                let one_minus_fract = 1.0 - fract;

                for c in 0..self.channels {
                    let s1 = if idx == 0 {
                        self.last_samples[c] as f32
                    } else {
                        input[(idx - 1) * self.channels + c] as f32
                    };
                    let s2 = if idx < num_frames {
                        input[idx * self.channels + c] as f32
                    } else {
                        input[(num_frames - 1) * self.channels + c] as f32
                    };
                    output.push((s1 * one_minus_fract + s2 * fract) as i16);
                }
                self.index += self.ratio;
            }
            self.index -= num_frames_f;
            if num_frames > 0 {
                let base = (num_frames - 1) * self.channels;
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        input.as_ptr().add(base),
                        self.last_samples.as_mut_ptr(),
                        self.channels,
                    );
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
        #[inline(always)]
        fn hermite(p: [f32; 4], t: f32) -> f32 {
            let c1 = 0.5 * (p[2] - p[0]);
            let c2 = p[0] - 2.5 * p[1] + 2.0 * p[2] - 0.5 * p[3];
            let c3 = 0.5 * (p[3] - p[0]) + 1.5 * (p[1] - p[2]);
            ((c3 * t + c2) * t + c1) * t + p[1]
        }
        pub fn process(&mut self, input: &[i16], output: &mut PooledBuffer) {
            let num_frames = input.len() / self.channels;
            let num_frames_f = num_frames as f32;
            let est_output = ((num_frames_f / self.ratio) as usize + 2) * self.channels;
            output.reserve(est_output);

            let last_frame_base = (num_frames - 1) * self.channels;

            while self.index < num_frames_f {
                let idx = self.index as usize;
                let t = self.index.fract();
                let base_idx = idx * self.channels;

                for ch in 0..self.channels {
                    let p0 = if idx == 0 {
                        self.last_samples[ch] as f32
                    } else {
                        input[base_idx - self.channels + ch] as f32
                    };
                    let p1 = input[base_idx + ch] as f32;
                    let p2 = if idx + 1 < num_frames {
                        input[base_idx + self.channels + ch] as f32
                    } else {
                        input[last_frame_base + ch] as f32
                    };
                    let p3 = if idx + 2 < num_frames {
                        input[base_idx + 2 * self.channels + ch] as f32
                    } else {
                        input[last_frame_base + ch] as f32
                    };
                    let s = Self::hermite([p0, p1, p2, p3], t)
                        .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                    output.push(s);
                }
                self.index += self.ratio;
            }
            self.index -= num_frames_f;
            if num_frames > 0 {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        input.as_ptr().add(last_frame_base),
                        self.last_samples.as_mut_ptr(),
                        self.channels,
                    );
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
use crate::audio::buffer::PooledBuffer;
pub use hermite::HermiteResampler;
pub use linear::LinearResampler;
pub use sinc::SincResampler;
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
