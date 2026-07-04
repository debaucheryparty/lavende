pub mod biquad {
    use std::f64::consts::PI;
    #[derive(Clone, Default)]
    pub struct BiquadCoeffs {
        pub b0: f64,
        pub b1: f64,
        pub b2: f64,
        pub a1: f64,
        pub a2: f64,
    }
    #[derive(Clone, Default)]
    pub struct BiquadState {
        pub x1: f64,
        pub x2: f64,
        pub y1: f64,
        pub y2: f64,
    }
    impl BiquadCoeffs {
        pub fn bandpass(freq: f64, q: f64, sample_rate: f64) -> Self {
            let omega0 = 2.0 * PI * freq / sample_rate;
            let sin_omega0 = omega0.sin();
            let cos_omega0 = omega0.cos();
            let alpha = sin_omega0 / (2.0 * q);
            let a0 = 1.0 + alpha;
            Self {
                b0: alpha / a0,
                b1: 0.0,
                b2: -alpha / a0,
                a1: -2.0 * cos_omega0 / a0,
                a2: (1.0 - alpha) / a0,
            }
        }
        pub fn lowpass(freq: f64, q: f64, sample_rate: f64) -> Self {
            let omega0 = 2.0 * PI * freq / sample_rate;
            let sin_omega0 = omega0.sin();
            let cos_omega0 = omega0.cos();
            let alpha = sin_omega0 / (2.0 * q);
            let a0 = 1.0 + alpha;
            let inv_a0 = 1.0 / a0;
            Self {
                b0: (1.0 - cos_omega0) * 0.5 * inv_a0,
                b1: (1.0 - cos_omega0) * inv_a0,
                b2: (1.0 - cos_omega0) * 0.5 * inv_a0,
                a1: -2.0 * cos_omega0 * inv_a0,
                a2: (1.0 - alpha) * inv_a0,
            }
        }
        pub fn highpass(freq: f64, q: f64, sample_rate: f64) -> Self {
            let omega0 = 2.0 * PI * freq / sample_rate;
            let sin_omega0 = omega0.sin();
            let cos_omega0 = omega0.cos();
            let alpha = sin_omega0 / (2.0 * q);
            let a0 = 1.0 + alpha;
            let inv_a0 = 1.0 / a0;
            Self {
                b0: (1.0 + cos_omega0) * 0.5 * inv_a0,
                b1: -(1.0 + cos_omega0) * inv_a0,
                b2: (1.0 + cos_omega0) * 0.5 * inv_a0,
                a1: -2.0 * cos_omega0 * inv_a0,
                a2: (1.0 - alpha) * inv_a0,
            }
        }
    }
    impl BiquadState {
        pub fn process(&mut self, input: f64, coeffs: &BiquadCoeffs) -> f64 {
            let output = coeffs.b0 * input + coeffs.b1 * self.x1 + coeffs.b2 * self.x2
                - coeffs.a1 * self.y1
                - coeffs.a2 * self.y2;
            if !output.is_finite() {
                self.reset();
                return 0.0;
            }
            self.x2 = self.x1;
            self.x1 = input;
            self.y2 = self.y1;
            self.y1 = output;
            output
        }
        pub fn reset(&mut self) {
            self.x1 = 0.0;
            self.x2 = 0.0;
            self.y1 = 0.0;
            self.y2 = 0.0;
        }
    }
}
pub mod channel_mix {
    use super::AudioFilter;
    pub struct ChannelMixFilter {
        left_to_left: f32,
        left_to_right: f32,
        right_to_left: f32,
        right_to_right: f32,
    }
    impl ChannelMixFilter {
        pub fn new(
            left_to_left: f32,
            left_to_right: f32,
            right_to_left: f32,
            right_to_right: f32,
        ) -> Self {
            Self {
                left_to_left: left_to_left.clamp(0.0, 1.0),
                left_to_right: left_to_right.clamp(0.0, 1.0),
                right_to_left: right_to_left.clamp(0.0, 1.0),
                right_to_right: right_to_right.clamp(0.0, 1.0),
            }
        }
    }
    impl AudioFilter for ChannelMixFilter {
        fn process(&mut self, samples: &mut [i16]) {
            let num_frames = samples.len() / 2;
            for frame in 0..num_frames {
                let offset = frame * 2;
                let left = samples[offset] as f32;
                let right = samples[offset + 1] as f32;
                let new_left = left * self.left_to_left + right * self.right_to_left;
                let new_right = left * self.left_to_right + right * self.right_to_right;
                samples[offset] = new_left.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                samples[offset + 1] = new_right.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            (self.left_to_left - 1.0).abs() > f32::EPSILON
                || self.left_to_right.abs() > f32::EPSILON
                || self.right_to_left.abs() > f32::EPSILON
                || (self.right_to_right - 1.0).abs() > f32::EPSILON
        }
        fn reset(&mut self) {}
    }
}
pub mod chorus {
    use super::{AudioFilter, delay_line::DelayLine, lfo::Lfo};
    use crate::audio::constants::TARGET_SAMPLE_RATE;
    const MAX_DELAY_MS: f32 = 50.0;
    const BUFFER_SIZE: usize = ((48000.0 * MAX_DELAY_MS) / 1000.0) as usize;
    pub struct ChorusFilter {
        rate: f32,
        depth: f32,
        delay: f32,
        mix: f32,
        feedback: f32,
        lfos: [Lfo; 4],
        delays: [DelayLine; 4],
    }
    impl ChorusFilter {
        pub fn new(rate: f32, depth: f32, delay: f32, mix: f32, feedback: f32) -> Self {
            let mut filter = Self {
                rate: 0.0,
                depth: 0.0,
                delay: 25.0,
                mix: 0.5,
                feedback: 0.0,
                lfos: [Lfo::new(), Lfo::new(), Lfo::new(), Lfo::new()],
                delays: [
                    DelayLine::new(BUFFER_SIZE),
                    DelayLine::new(BUFFER_SIZE),
                    DelayLine::new(BUFFER_SIZE),
                    DelayLine::new(BUFFER_SIZE),
                ],
            };
            filter.set_lfo_phases();
            filter.update(rate, depth, delay, mix, feedback);
            filter
        }
        fn set_lfo_phases(&mut self) {
            self.lfos[0].set_phase(0.0);
            self.lfos[1].set_phase(std::f64::consts::PI / 2.0);
            self.lfos[2].set_phase(std::f64::consts::PI);
            self.lfos[3].set_phase(3.0 * std::f64::consts::PI / 2.0);
        }
        pub fn update(&mut self, rate: f32, depth: f32, delay: f32, mix: f32, feedback: f32) {
            self.rate = rate;
            self.depth = depth.clamp(0.0, 1.0);
            self.delay = delay.clamp(1.0, MAX_DELAY_MS - 5.0);
            self.mix = mix.clamp(0.0, 1.0);
            self.feedback = feedback.clamp(0.0, 0.95);
            let rate2 = self.rate * 1.1;
            self.lfos[0].update(self.rate.into(), self.depth.into());
            self.lfos[1].update(self.rate.into(), self.depth.into());
            self.lfos[2].update(rate2.into(), self.depth.into());
            self.lfos[3].update(rate2.into(), self.depth.into());
        }
    }
    impl AudioFilter for ChorusFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.rate == 0.0 || self.depth == 0.0 || self.mix == 0.0 {
                return;
            }
            let fs = TARGET_SAMPLE_RATE as f32;
            let delay_width = self.depth * (fs * 0.004);
            let center_delay_samples = self.delay * (fs / 1000.0);
            let center_delay_samples2 = center_delay_samples * 1.2;
            for chunk in samples.chunks_exact_mut(2) {
                let left_in = chunk[0] as f32;
                let right_in = chunk[1] as f32;
                let lfo1_l = self.lfos[0].get_value() as f32;
                let lfo1_r = self.lfos[1].get_value() as f32;
                let delay1_l = center_delay_samples + lfo1_l * delay_width;
                let delay1_r = center_delay_samples + lfo1_r * delay_width;
                let delayed1_l = self.delays[0].read(delay1_l);
                let delayed1_r = self.delays[1].read(delay1_r);
                let lfo2_l = self.lfos[2].get_value() as f32;
                let lfo2_r = self.lfos[3].get_value() as f32;
                let delay2_l = center_delay_samples2 + lfo2_l * delay_width;
                let delay2_r = center_delay_samples2 + lfo2_r * delay_width;
                let delayed2_l = self.delays[2].read(delay2_l);
                let delayed2_r = self.delays[3].read(delay2_r);
                let wet_left = (delayed1_l + delayed2_l) * 0.5;
                let wet_right = (delayed1_r + delayed2_r) * 0.5;
                let final_left = left_in * (1.0 - self.mix) + wet_left * self.mix;
                let final_right = right_in * (1.0 - self.mix) + wet_right * self.mix;
                self.delays[0].write(
                    (left_in + delayed1_l * self.feedback).clamp(i16::MIN as f32, i16::MAX as f32),
                );
                self.delays[1].write(
                    (right_in + delayed1_r * self.feedback).clamp(i16::MIN as f32, i16::MAX as f32),
                );
                self.delays[2].write(
                    (left_in + delayed2_l * self.feedback).clamp(i16::MIN as f32, i16::MAX as f32),
                );
                self.delays[3].write(
                    (right_in + delayed2_r * self.feedback).clamp(i16::MIN as f32, i16::MAX as f32),
                );
                chunk[0] = final_left.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                chunk[1] = final_right.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.rate > 0.0 && self.depth > 0.0 && self.mix > 0.0
        }
        fn reset(&mut self) {
            for delay in &mut self.delays {
                delay.clear();
            }
            self.set_lfo_phases();
        }
    }
}
pub mod compressor {
    use super::AudioFilter;
    fn db_to_gain(db: f32) -> f32 {
        10f32.powf(db / 20.0)
    }
    fn gain_to_db(gain: f32) -> f32 {
        20.0 * gain.max(1e-10).log10()
    }
    pub struct CompressorFilter {
        threshold: f32,
        ratio: f32,
        makeup_gain: f32,
        envelope: f32,
        attack_coef: f32,
        release_coef: f32,
    }
    impl CompressorFilter {
        pub fn new(
            threshold: f32,
            ratio: f32,
            attack: f32,
            release: f32,
            makeup_gain: f32,
        ) -> Self {
            let attack = attack.max(0.001);
            let release = release.max(0.01);
            Self {
                threshold,
                ratio: ratio.max(1.0),
                makeup_gain,
                envelope: 0.0,
                attack_coef: (-1.0 / (attack * 48000.0)).exp(),
                release_coef: (-1.0 / (release * 48000.0)).exp(),
            }
        }
    }
    impl AudioFilter for CompressorFilter {
        fn process(&mut self, samples: &mut [i16]) {
            let makeup_gain = db_to_gain(self.makeup_gain);
            for chunk in samples.chunks_exact_mut(2) {
                let left_in = chunk[0] as f32 / 32768.0;
                let right_in = chunk[1] as f32 / 32768.0;
                let abs_sample = left_in.abs().max(right_in.abs());
                if abs_sample > self.envelope {
                    self.envelope = self.attack_coef * (self.envelope - abs_sample) + abs_sample;
                } else {
                    self.envelope = self.release_coef * (self.envelope - abs_sample) + abs_sample;
                }
                let envelope_db = gain_to_db(self.envelope);
                let mut reduction_db = 0.0;
                if envelope_db > self.threshold {
                    reduction_db = (self.threshold - envelope_db) * (1.0 - 1.0 / self.ratio);
                }
                let gain = db_to_gain(reduction_db) * makeup_gain;
                chunk[0] =
                    (left_in * gain * 32768.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                chunk[1] =
                    (right_in * gain * 32768.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.threshold < 0.0 || self.ratio > 1.0 || self.makeup_gain != 0.0
        }
        fn reset(&mut self) {
            self.envelope = 0.0;
        }
    }
}
pub mod delay_line {
    pub struct DelayLine {
        buffer: Vec<f32>,
        size: usize,
        write_index: usize,
    }
    impl DelayLine {
        pub fn new(size: usize) -> Self {
            Self {
                buffer: vec![0.0; size],
                size,
                write_index: 0,
            }
        }
        pub fn write(&mut self, sample: f32) {
            self.buffer[self.write_index] = sample;
            self.write_index = (self.write_index + 1) % self.size;
        }
        pub fn read(&self, delay_in_samples: f32) -> f32 {
            let safe_delay = delay_in_samples.max(0.0).min((self.size - 1) as f32);
            let int_delay = safe_delay as usize;
            let frac = safe_delay - int_delay as f32;
            let idx0 = (self.write_index + self.size - int_delay) % self.size;
            let idx1 = (self.write_index + self.size - int_delay - 1) % self.size;
            let s0 = self.buffer[idx0];
            let s1 = self.buffer[idx1];
            s0 * (1.0 - frac) + s1 * frac
        }
        pub fn clear(&mut self) {
            self.buffer.fill(0.0);
        }
    }
}
pub mod distortion {
    use super::AudioFilter;
    use crate::audio::constants::INT16_NORM_F64;
    pub struct DistortionFilter {
        sin_offset: f32,
        sin_scale: f32,
        cos_offset: f32,
        cos_scale: f32,
        tan_offset: f32,
        tan_scale: f32,
        offset: f32,
        scale: f32,
    }
    impl DistortionFilter {
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            sin_offset: f32,
            sin_scale: f32,
            cos_offset: f32,
            cos_scale: f32,
            tan_offset: f32,
            tan_scale: f32,
            offset: f32,
            scale: f32,
        ) -> Self {
            Self {
                sin_offset,
                sin_scale,
                cos_offset,
                cos_scale,
                tan_offset,
                tan_scale,
                offset,
                scale,
            }
        }
    }
    impl AudioFilter for DistortionFilter {
        fn process(&mut self, samples: &mut [i16]) {
            let num_frames = samples.len() / 2;
            for frame in 0..num_frames {
                let offset_idx = frame * 2;
                for ch in 0..2 {
                    let sample = samples[offset_idx + ch] as f64;
                    let normalized = sample / INT16_NORM_F64;
                    let mut distorted = 0.0f64;
                    if self.sin_scale != 0.0 {
                        distorted +=
                            (normalized * self.sin_scale as f64 + self.sin_offset as f64).sin();
                    }
                    if self.cos_scale != 0.0 {
                        distorted +=
                            (normalized * self.cos_scale as f64 + self.cos_offset as f64).cos();
                    }
                    if self.tan_scale != 0.0 {
                        let tan_input =
                            (normalized * self.tan_scale as f64 + self.tan_offset as f64).clamp(
                                -std::f64::consts::FRAC_PI_2 + 0.01,
                                std::f64::consts::FRAC_PI_2 - 0.01,
                            );
                        distorted += tan_input.tan();
                    }
                    distorted =
                        (distorted * self.scale as f64 + self.offset as f64) * INT16_NORM_F64;
                    samples[offset_idx + ch] =
                        distorted.clamp(i16::MIN as f64, i16::MAX as f64) as i16;
                }
            }
        }
        fn is_enabled(&self) -> bool {
            self.sin_offset != 0.0
                || self.sin_scale != 0.0
                || self.cos_offset != 0.0
                || self.cos_scale != 0.0
                || self.tan_offset != 0.0
                || self.tan_scale != 0.0
                || self.offset != 0.0
                || (self.scale - 1.0).abs() > f32::EPSILON
        }
        fn reset(&mut self) {}
    }
}
pub mod echo {
    use super::AudioFilter;
    use crate::audio::constants::TARGET_SAMPLE_RATE;
    use std::collections::VecDeque;
    pub struct EchoFilter {
        echo_length: f32,
        decay: f32,
        buffer: VecDeque<i16>,
        delay_samples: usize,
    }
    impl EchoFilter {
        pub fn new(echo_length: f32, decay: f32) -> Self {
            let length = echo_length.clamp(0.001, 5.0);
            let decay = decay.clamp(0.0, 1.0);
            let frames = (TARGET_SAMPLE_RATE as f32 * length) as usize;
            let samples = frames * 2;
            let mut buffer = VecDeque::with_capacity(samples);
            buffer.extend(std::iter::repeat_n(0, samples));
            Self {
                echo_length: length,
                decay,
                buffer,
                delay_samples: samples,
            }
        }
    }
    impl AudioFilter for EchoFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.echo_length <= 0.0 || self.decay <= 0.0 {
                return;
            }
            for sample in samples.iter_mut() {
                let delayed = self.buffer.pop_front().unwrap_or(0);
                let mixed = (*sample as f32) + (delayed as f32 * self.decay);
                let out = mixed.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                *sample = out;
                self.buffer.push_back(out);
            }
        }
        fn is_enabled(&self) -> bool {
            self.echo_length > 0.0 && self.decay > 0.0
        }
        fn reset(&mut self) {
            self.buffer.clear();
            self.buffer
                .extend(std::iter::repeat_n(0, self.delay_samples));
        }
    }
}
pub mod equalizer {
    use super::AudioFilter;
    const BAND_COUNT: usize = 15;
    const DEFAULT_MAKEUP_GAIN: f32 = 4.0;
    struct Coefficients {
        beta: f32,
        alpha: f32,
        gamma: f32,
    }
    #[allow(clippy::excessive_precision)]
    const COEFFICIENTS_48000: [Coefficients; BAND_COUNT] = [
        Coefficients {
            beta: 9.9847546664e-01,
            alpha: 7.6226668143e-04,
            gamma: 1.9984647656e+00,
        },
        Coefficients {
            beta: 9.9756184654e-01,
            alpha: 1.2190767289e-03,
            gamma: 1.9975344645e+00,
        },
        Coefficients {
            beta: 9.9616261379e-01,
            alpha: 1.9186931041e-03,
            gamma: 1.9960947369e+00,
        },
        Coefficients {
            beta: 9.9391578543e-01,
            alpha: 3.0421072865e-03,
            gamma: 1.9937449618e+00,
        },
        Coefficients {
            beta: 9.9028307215e-01,
            alpha: 4.8584639242e-03,
            gamma: 1.9898465702e+00,
        },
        Coefficients {
            beta: 9.8485897264e-01,
            alpha: 7.5705136795e-03,
            gamma: 1.9837962543e+00,
        },
        Coefficients {
            beta: 9.7588512657e-01,
            alpha: 1.2057436715e-02,
            gamma: 1.9731772447e+00,
        },
        Coefficients {
            beta: 9.6228521814e-01,
            alpha: 1.8857390928e-02,
            gamma: 1.9556164694e+00,
        },
        Coefficients {
            beta: 9.4080933132e-01,
            alpha: 2.9595334338e-02,
            gamma: 1.9242054384e+00,
        },
        Coefficients {
            beta: 9.0702059196e-01,
            alpha: 4.6489704022e-02,
            gamma: 1.8653476166e+00,
        },
        Coefficients {
            beta: 8.5868004289e-01,
            alpha: 7.0659978553e-02,
            gamma: 1.7600401337e+00,
        },
        Coefficients {
            beta: 7.8409610788e-01,
            alpha: 1.0795194606e-01,
            gamma: 1.5450725522e+00,
        },
        Coefficients {
            beta: 6.8332861002e-01,
            alpha: 1.5833569499e-01,
            gamma: 1.1426447155e+00,
        },
        Coefficients {
            beta: 5.5267518228e-01,
            alpha: 2.2366240886e-01,
            gamma: 4.0186190803e-01,
        },
        Coefficients {
            beta: 4.1811888447e-01,
            alpha: 2.9094055777e-01,
            gamma: -7.0905944223e-01,
        },
    ];
    #[derive(Clone, Default)]
    struct EqBandState {
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
    }
    impl EqBandState {
        fn process(&mut self, sample: f32, coeffs: &Coefficients) -> f32 {
            let result =
                coeffs.alpha * (sample - self.x2) + coeffs.gamma * self.y1 - coeffs.beta * self.y2;
            self.x2 = self.x1;
            self.x1 = sample;
            self.y2 = self.y1;
            if !result.is_finite() {
                self.y1 = 0.0;
                return 0.0;
            }
            self.y1 = result;
            result
        }
        fn reset(&mut self) {
            self.x1 = 0.0;
            self.x2 = 0.0;
            self.y1 = 0.0;
            self.y2 = 0.0;
        }
    }
    pub struct EqualizerFilter {
        gains: [f32; BAND_COUNT],
        states: [[EqBandState; 2]; BAND_COUNT],
        makeup_gain: f32,
    }
    impl EqualizerFilter {
        pub fn new(bands: &[(u8, f32)]) -> Self {
            let mut gains = [0.0f32; BAND_COUNT];
            for &(band, gain) in bands {
                if (band as usize) < BAND_COUNT {
                    gains[band as usize] = gain.clamp(-0.25, 1.0);
                }
            }
            let states: [[EqBandState; 2]; BAND_COUNT] =
                std::array::from_fn(|_| [EqBandState::default(), EqBandState::default()]);
            Self {
                gains,
                states,
                makeup_gain: DEFAULT_MAKEUP_GAIN,
            }
        }
    }
    impl AudioFilter for EqualizerFilter {
        fn process(&mut self, samples: &mut [i16]) {
            let num_frames = samples.len() / 2;
            for frame in 0..num_frames {
                let offset = frame * 2;
                let left_f = samples[offset] as f32 / 32768.0;
                let right_f = samples[offset + 1] as f32 / 32768.0;
                let mut result_left = left_f * 0.25;
                let mut result_right = right_f * 0.25;
                for (b, coeffs) in COEFFICIENTS_48000.iter().enumerate() {
                    let gain = self.gains[b];
                    if gain.abs() < f32::EPSILON {
                        self.states[b][0].process(left_f, coeffs);
                        self.states[b][1].process(right_f, coeffs);
                        continue;
                    }
                    let band_left = self.states[b][0].process(left_f, coeffs);
                    let band_right = self.states[b][1].process(right_f, coeffs);
                    result_left += band_left * gain;
                    result_right += band_right * gain;
                }
                let out_left = (result_left * self.makeup_gain).clamp(-1.0, 1.0);
                let out_right = (result_right * self.makeup_gain).clamp(-1.0, 1.0);
                samples[offset] = (out_left * 32767.0) as i16;
                samples[offset + 1] = (out_right * 32767.0) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.gains.iter().any(|g| g.abs() > f32::EPSILON)
        }
        fn reset(&mut self) {
            for band_states in self.states.iter_mut() {
                for state in band_states.iter_mut() {
                    state.reset();
                }
            }
        }
    }
}
pub mod flanger {
    use super::{AudioFilter, delay_line::DelayLine, lfo::Lfo};
    use crate::audio::constants::TARGET_SAMPLE_RATE;
    const MAX_DELAY_MS: f32 = 10.0;
    const BUFFER_SIZE: usize = ((48000.0 * MAX_DELAY_MS) / 1000.0) as usize;
    pub struct FlangerFilter {
        rate: f32,
        depth: f32,
        feedback: f32,
        lfo: Lfo,
        delay_line: DelayLine,
    }
    impl FlangerFilter {
        pub fn new(rate: f32, depth: f32, feedback: f32) -> Self {
            let mut filter = Self {
                rate: 0.0,
                depth: 0.0,
                feedback: 0.0,
                lfo: Lfo::new(),
                delay_line: DelayLine::new(BUFFER_SIZE),
            };
            filter.update(rate, depth, feedback);
            filter
        }
        pub fn update(&mut self, rate: f32, depth: f32, feedback: f32) {
            self.rate = rate;
            self.depth = depth.clamp(0.0, 1.0);
            self.feedback = feedback.clamp(0.0, 0.95);
            self.lfo.update(self.rate as f64, self.depth as f64);
        }
    }
    impl AudioFilter for FlangerFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.rate == 0.0 || self.depth == 0.0 {
                return;
            }
            let fs = TARGET_SAMPLE_RATE as f32;
            let max_delay_width = self.depth * (fs * 0.005);
            let center_delay = max_delay_width;
            for sample in samples.iter_mut() {
                let lfo_value = self.lfo.get_value() as f32;
                let delay = center_delay + lfo_value * max_delay_width;
                let delayed = self.delay_line.read(delay);
                let input = (*sample as f32) + delayed * self.feedback;
                self.delay_line
                    .write(input.clamp(i16::MIN as f32, i16::MAX as f32));
                let output = (*sample as f32) + delayed;
                *sample = output.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.rate > 0.0 && self.depth > 0.0
        }
        fn reset(&mut self) {
            self.delay_line.clear();
            self.lfo.set_phase(0.0);
        }
    }
}
pub mod high_pass {
    use super::{
        AudioFilter,
        biquad::{BiquadCoeffs, BiquadState},
    };
    use crate::audio::constants::TARGET_SAMPLE_RATE;
    pub struct HighPassFilter {
        cutoff_frequency: i32,
        boost_factor: f32,
        left_state: BiquadState,
        right_state: BiquadState,
        coeffs: Option<BiquadCoeffs>,
    }
    impl HighPassFilter {
        pub fn new(cutoff_frequency: i32, boost_factor: f32) -> Self {
            let mut filter = Self {
                cutoff_frequency,
                boost_factor,
                left_state: BiquadState::default(),
                right_state: BiquadState::default(),
                coeffs: None,
            };
            filter.update_coefficients();
            filter
        }
        fn update_coefficients(&mut self) {
            if self.cutoff_frequency <= 0 {
                return;
            }
            let fs = TARGET_SAMPLE_RATE as f64;
            let fc = self.cutoff_frequency as f64;
            let q = 0.7071067811865475;
            let w0 = 2.0 * std::f64::consts::PI * (fc / fs);
            let cos_w0 = w0.cos();
            let sin_w0 = w0.sin();
            let alpha = sin_w0 / (2.0 * q);
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            let b0 = (1.0 + cos_w0) / 2.0;
            let b1 = -(1.0 + cos_w0);
            let b2 = (1.0 + cos_w0) / 2.0;
            self.coeffs = Some(BiquadCoeffs {
                b0: b0 / a0,
                b1: b1 / a0,
                b2: b2 / a0,
                a1: a1 / a0,
                a2: a2 / a0,
            });
        }
    }
    impl AudioFilter for HighPassFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.cutoff_frequency <= 0 {
                return;
            }
            let coeffs = match &self.coeffs {
                Some(c) => c,
                None => return,
            };
            for chunk in samples.chunks_exact_mut(2) {
                let left_in = chunk[0] as f32;
                let right_in = chunk[1] as f32;
                let left_out =
                    self.left_state.process(left_in as f64, coeffs) as f32 * self.boost_factor;
                let right_out =
                    self.right_state.process(right_in as f64, coeffs) as f32 * self.boost_factor;
                chunk[0] = left_out.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                chunk[1] = right_out.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.cutoff_frequency > 0
        }
        fn reset(&mut self) {
            self.left_state.reset();
            self.right_state.reset();
        }
    }
}
pub mod karaoke {
    use super::{
        AudioFilter,
        biquad::{BiquadCoeffs, BiquadState},
    };
    use crate::audio::constants::{MIX_BUFFER_SIZE, TARGET_SAMPLE_RATE};
    const MAX_OUTPUT_GAIN: f64 = crate::audio::constants::KARAOKE_MAX_OUTPUT_GAIN;
    pub struct KaraokeFilter {
        level: f32,
        mono_level: f32,
        filter_band: f32,
        filter_width: f32,
        lp_coeffs: BiquadCoeffs,
        hp_coeffs: BiquadCoeffs,
        lp_states: [BiquadState; 2],
        hp_states: [BiquadState; 2],
        prev_gain: f64,
        out_left_buf: Vec<f64>,
        out_right_buf: Vec<f64>,
    }
    impl KaraokeFilter {
        pub fn new(level: f32, mono_level: f32, filter_band: f32, filter_width: f32) -> Self {
            let level = level.clamp(0.0, 1.0);
            let mono_level = mono_level.clamp(0.0, 1.0);
            let (lp_coeffs, hp_coeffs) =
                Self::compute_coefficients(filter_band as f64, filter_width as f64);
            Self {
                level,
                mono_level,
                filter_band,
                filter_width,
                lp_coeffs,
                hp_coeffs,
                lp_states: [BiquadState::default(), BiquadState::default()],
                hp_states: [BiquadState::default(), BiquadState::default()],
                prev_gain: MAX_OUTPUT_GAIN,
                out_left_buf: Vec::with_capacity(MIX_BUFFER_SIZE),
                out_right_buf: Vec::with_capacity(MIX_BUFFER_SIZE),
            }
        }
        fn compute_coefficients(band: f64, width: f64) -> (BiquadCoeffs, BiquadCoeffs) {
            let fs = TARGET_SAMPLE_RATE as f64;
            if band <= 0.0 || width <= 0.0 {
                let passthrough = BiquadCoeffs {
                    b0: 1.0,
                    b1: 0.0,
                    b2: 0.0,
                    a1: 0.0,
                    a2: 0.0,
                };
                return (passthrough.clone(), passthrough);
            }
            let fc = band.clamp(1.0, fs * 0.49);
            let w = width.max(1e-6);
            let q = (fc / w).max(1e-4);
            let lp = BiquadCoeffs::lowpass(fc, q, fs);
            let hp = BiquadCoeffs::highpass(fc, q, fs);
            (lp, hp)
        }
    }
    impl AudioFilter for KaraokeFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.level <= 0.0 && self.mono_level <= 0.0 {
                return;
            }
            let num_frames = samples.len() / 2;
            if num_frames == 0 {
                return;
            }
            if self.out_left_buf.len() < num_frames {
                self.out_left_buf.resize(num_frames, 0.0);
                self.out_right_buf.resize(num_frames, 0.0);
            }
            let do_filter = self.level > 0.0 && self.filter_band > 0.0 && self.filter_width > 0.0;
            let mut original_energy = 0.0f64;
            let mut processed_energy = 0.0f64;
            for frame in 0..num_frames {
                let offset = frame * 2;
                let mut left = samples[offset] as f64 * crate::audio::constants::KARAOKE_INV;
                let mut right = samples[offset + 1] as f64 * crate::audio::constants::KARAOKE_INV;
                original_energy += left * left + right * right;
                if self.mono_level > 0.0 {
                    let mid = (left + right) * 0.5;
                    let sub = mid * self.mono_level as f64;
                    left -= sub;
                    right -= sub;
                }
                if do_filter {
                    let low_left = self.lp_states[0].process(left, &self.lp_coeffs);
                    let low_right = self.lp_states[1].process(right, &self.lp_coeffs);
                    let high_left = self.hp_states[0].process(left, &self.hp_coeffs);
                    let high_right = self.hp_states[1].process(right, &self.hp_coeffs);
                    let cancelled = high_left - high_right;
                    left = low_left + cancelled * self.level as f64;
                    right = low_right + cancelled * self.level as f64;
                }
                self.out_left_buf[frame] = left;
                self.out_right_buf[frame] = right;
                processed_energy += left * left + right * right;
            }
            let denom = (num_frames * 2) as f64;
            original_energy /= denom;
            processed_energy /= denom;
            let gain = if processed_energy > 1e-15 {
                let g = (original_energy.max(1e-12) / processed_energy).sqrt();
                g.min(MAX_OUTPUT_GAIN)
            } else {
                MAX_OUTPUT_GAIN
            };
            let smooth = if gain > self.prev_gain { 0.06 } else { 0.3 };
            let target = self.prev_gain + (gain - self.prev_gain) * smooth;
            let step = (target - self.prev_gain) / num_frames as f64;
            let mut current = self.prev_gain;
            for frame in 0..num_frames {
                let offset = frame * 2;
                current += step;
                let mut out_l = self.out_left_buf[frame] * current;
                let mut out_r = self.out_right_buf[frame] * current;
                let peak = out_l.abs().max(out_r.abs());
                if peak > 0.9999 {
                    let s = 0.9999 / peak;
                    out_l *= s;
                    out_r *= s;
                }
                samples[offset] = ((out_l * crate::audio::constants::KARAOKE_SCALE).round() as i32)
                    .clamp(i16::MIN as i32, i16::MAX as i32)
                    as i16;
                samples[offset + 1] =
                    ((out_r * crate::audio::constants::KARAOKE_SCALE).round() as i32)
                        .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
            self.prev_gain = target;
        }
        fn is_enabled(&self) -> bool {
            self.level > 0.0 || self.mono_level > 0.0
        }
        fn reset(&mut self) {
            for s in self.lp_states.iter_mut() {
                s.reset();
            }
            for s in self.hp_states.iter_mut() {
                s.reset();
            }
            self.prev_gain = MAX_OUTPUT_GAIN;
        }
    }
}
pub mod lfo {
    use crate::audio::constants::TARGET_SAMPLE_RATE;
    use std::f64::consts::PI;
    const TWO_PI: f64 = 2.0 * PI;
    #[derive(Default)]
    pub struct Lfo {
        phase: f64,
        pub frequency: f64,
        pub depth: f64,
    }
    impl Lfo {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn update(&mut self, frequency: f64, depth: f64) {
            self.frequency = frequency;
            self.depth = depth;
        }
        pub fn get_value(&mut self) -> f64 {
            if self.frequency == 0.0 {
                return 0.0;
            }
            let value = self.phase.sin();
            self.phase += TWO_PI * self.frequency / TARGET_SAMPLE_RATE as f64;
            if self.phase > TWO_PI {
                self.phase -= TWO_PI;
            }
            value
        }
        pub fn process(&mut self) -> f64 {
            if self.depth == 0.0 || self.frequency == 0.0 {
                return 1.0;
            }
            let lfo_value = self.get_value();
            let normalized = (lfo_value + 1.0) / 2.0;
            1.0 - self.depth * normalized
        }
        pub fn reset(&mut self) {
            self.phase = 0.0;
        }
        pub fn set_phase(&mut self, phase: f64) {
            self.phase = phase;
        }
    }
}
pub mod low_pass {
    use super::AudioFilter;
    pub struct LowPassFilter {
        smoothing: f32,
        smoothing_factor: f64,
        prev_left: f64,
        prev_right: f64,
    }
    impl LowPassFilter {
        pub fn new(smoothing: f32) -> Self {
            let smoothing_factor = if smoothing > 1.0 {
                1.0 / smoothing as f64
            } else {
                0.0
            };
            Self {
                smoothing,
                smoothing_factor,
                prev_left: 0.0,
                prev_right: 0.0,
            }
        }
    }
    impl AudioFilter for LowPassFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.smoothing <= 1.0 {
                return;
            }
            let num_frames = samples.len() / 2;
            for frame in 0..num_frames {
                let offset = frame * 2;
                let left = samples[offset] as f64;
                let new_left = self.prev_left + self.smoothing_factor * (left - self.prev_left);
                self.prev_left = new_left;
                samples[offset] = new_left.clamp(i16::MIN as f64, i16::MAX as f64) as i16;
                let right = samples[offset + 1] as f64;
                let new_right = self.prev_right + self.smoothing_factor * (right - self.prev_right);
                self.prev_right = new_right;
                samples[offset + 1] = new_right.clamp(i16::MIN as f64, i16::MAX as f64) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.smoothing > 1.0
        }
        fn reset(&mut self) {
            self.prev_left = 0.0;
            self.prev_right = 0.0;
        }
    }
}
pub mod normalization {
    use super::AudioFilter;
    use crate::audio::constants::TARGET_SAMPLE_RATE;
    pub struct NormalizationFilter {
        max_amplitude: f32,
        adaptive: bool,
        envelope: f32,
        attack_coef: f32,
        release_coef: f32,
    }
    impl NormalizationFilter {
        pub fn new(max_amplitude: f32, adaptive: bool) -> Self {
            let max_amplitude = max_amplitude.max(0.01);
            let attack_ms = 1.0;
            let release_ms = 100.0;
            let attack_coef =
                (-1.0 / ((attack_ms / 1000.0) * TARGET_SAMPLE_RATE as f64) as f32).exp();
            let release_coef =
                (-1.0 / ((release_ms / 1000.0) * TARGET_SAMPLE_RATE as f64) as f32).exp();
            Self {
                max_amplitude,
                adaptive,
                envelope: 0.0,
                attack_coef,
                release_coef,
            }
        }
    }
    impl AudioFilter for NormalizationFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.max_amplitude <= 0.0 {
                return;
            }
            if !self.adaptive {
                for sample in samples.iter_mut() {
                    let v = (*sample as f32) / 32768.0;
                    let scaled = v.clamp(-self.max_amplitude, self.max_amplitude);
                    *sample = (scaled * 32768.0) as i16;
                }
            } else {
                for chunk in samples.chunks_exact_mut(2) {
                    let left_in = chunk[0] as f32 / 32768.0;
                    let right_in = chunk[1] as f32 / 32768.0;
                    let abs_peak = left_in.abs().max(right_in.abs());
                    if abs_peak > self.envelope {
                        self.envelope = self.attack_coef * (self.envelope - abs_peak) + abs_peak;
                    } else {
                        self.envelope = self.release_coef * (self.envelope - abs_peak) + abs_peak;
                    }
                    let envelope_safe = self.envelope.max(0.001);
                    let gain = if envelope_safe > self.max_amplitude {
                        self.max_amplitude / envelope_safe
                    } else {
                        1.0
                    };
                    chunk[0] =
                        (left_in * gain * 32768.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                    chunk[1] =
                        (right_in * gain * 32768.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                }
            }
        }
        fn is_enabled(&self) -> bool {
            self.max_amplitude > 0.0
        }
        fn reset(&mut self) {
            self.envelope = 0.0;
        }
    }
}
pub mod phaser {
    use super::{AudioFilter, lfo::Lfo};
    use crate::audio::constants::TARGET_SAMPLE_RATE;
    const MAX_STAGES: usize = 12;
    struct Allpass {
        a1: f32,
        z1: f32,
    }
    impl Allpass {
        fn new() -> Self {
            Self { a1: 0.0, z1: 0.0 }
        }
        fn set_coefficient(&mut self, coef: f32) {
            self.a1 = coef;
        }
        fn process(&mut self, input: f32) -> f32 {
            let output = input * -self.a1 + self.z1;
            self.z1 = output * self.a1 + input;
            output
        }
        fn reset(&mut self) {
            self.z1 = 0.0;
        }
    }
    pub struct PhaserFilter {
        stages: usize,
        rate: f32,
        depth: f32,
        feedback: f32,
        mix: f32,
        min_frequency: f32,
        max_frequency: f32,
        left_lfo: Lfo,
        right_lfo: Lfo,
        left_filters: Vec<Allpass>,
        right_filters: Vec<Allpass>,
        last_left_feedback: f32,
        last_right_feedback: f32,
    }
    impl PhaserFilter {
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            stages: i32,
            rate: f32,
            depth: f32,
            feedback: f32,
            mix: f32,
            min_frequency: f32,
            max_frequency: f32,
        ) -> Self {
            let mut left_filters = Vec::with_capacity(MAX_STAGES);
            let mut right_filters = Vec::with_capacity(MAX_STAGES);
            for _ in 0..MAX_STAGES {
                left_filters.push(Allpass::new());
                right_filters.push(Allpass::new());
            }
            let mut right_lfo = Lfo::new();
            right_lfo.set_phase(std::f64::consts::PI / 2.0);
            let mut filter = Self {
                stages: 4,
                rate: 0.0,
                depth: 1.0,
                feedback: 0.0,
                mix: 0.5,
                min_frequency: 100.0,
                max_frequency: 2500.0,
                left_lfo: Lfo::new(),
                right_lfo,
                left_filters,
                right_filters,
                last_left_feedback: 0.0,
                last_right_feedback: 0.0,
            };
            filter.update(
                stages,
                rate,
                depth,
                feedback,
                mix,
                min_frequency,
                max_frequency,
            );
            filter
        }
        #[allow(clippy::too_many_arguments)]
        pub fn update(
            &mut self,
            stages: i32,
            rate: f32,
            depth: f32,
            feedback: f32,
            mix: f32,
            min_frequency: f32,
            max_frequency: f32,
        ) {
            self.stages = (stages as usize).clamp(2, MAX_STAGES);
            self.rate = rate;
            self.depth = depth.clamp(0.0, 1.0);
            self.feedback = feedback.clamp(0.0, 0.9);
            self.mix = mix.clamp(0.0, 1.0);
            self.min_frequency = min_frequency;
            self.max_frequency = max_frequency;
            self.left_lfo.update(self.rate as f64, self.depth as f64);
            self.right_lfo.update(self.rate as f64, self.depth as f64);
        }
    }
    impl AudioFilter for PhaserFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.rate == 0.0 || self.depth == 0.0 || self.mix == 0.0 {
                return;
            }
            let fs = TARGET_SAMPLE_RATE as f32;
            let sweep_range = self.max_frequency - self.min_frequency;
            for chunk in samples.chunks_exact_mut(2) {
                let left_sample = chunk[0] as f32;
                let right_sample = chunk[1] as f32;
                let left_lfo_val = (self.left_lfo.get_value() as f32 + 1.0) / 2.0;
                let right_lfo_val = (self.right_lfo.get_value() as f32 + 1.0) / 2.0;
                let current_left_freq = self.min_frequency + sweep_range * left_lfo_val;
                let current_right_freq = self.min_frequency + sweep_range * right_lfo_val;
                let tan_left = (std::f32::consts::PI * current_left_freq / fs).tan();
                let a_left = (1.0 - tan_left) / (1.0 + tan_left);
                let tan_right = (std::f32::consts::PI * current_right_freq / fs).tan();
                let a_right = (1.0 - tan_right) / (1.0 + tan_right);
                let mut wet_left = left_sample + self.last_left_feedback * self.feedback;
                for j in 0..self.stages {
                    self.left_filters[j].set_coefficient(a_left);
                    wet_left = self.left_filters[j].process(wet_left);
                }
                self.last_left_feedback = wet_left;
                let final_left = left_sample * (1.0 - self.mix) + wet_left * self.mix;
                let mut wet_right = right_sample + self.last_right_feedback * self.feedback;
                for j in 0..self.stages {
                    self.right_filters[j].set_coefficient(a_right);
                    wet_right = self.right_filters[j].process(wet_right);
                }
                self.last_right_feedback = wet_right;
                let final_right = right_sample * (1.0 - self.mix) + wet_right * self.mix;
                chunk[0] = final_left.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                chunk[1] = final_right.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.rate > 0.0 && self.depth > 0.0 && self.mix > 0.0
        }
        fn reset(&mut self) {
            for filter in &mut self.left_filters {
                filter.reset();
            }
            for filter in &mut self.right_filters {
                filter.reset();
            }
            self.last_left_feedback = 0.0;
            self.last_right_feedback = 0.0;
            self.left_lfo.set_phase(0.0);
            self.right_lfo.set_phase(std::f64::consts::PI / 2.0);
        }
    }
}
pub mod phonograph {
    use super::{
        AudioFilter,
        biquad::{BiquadCoeffs, BiquadState},
        delay_line::DelayLine,
        lfo::Lfo,
    };
    use crate::audio::constants::{
        PHONOGRAPH_MAX_DELAY_MS, PHONOGRAPH_R1_SIZE, PHONOGRAPH_R2_SIZE, PHONOGRAPH_R3_SIZE,
        TARGET_SAMPLE_RATE,
    };
    const BUFFER_SIZE: usize =
        ((TARGET_SAMPLE_RATE as f32 * PHONOGRAPH_MAX_DELAY_MS) / 1000.0) as usize;
    struct XorShift32 {
        s: u32,
    }
    impl XorShift32 {
        fn new(seed: u32) -> Self {
            Self { s: seed }
        }
        fn next_u32(&mut self) -> u32 {
            let mut x = self.s;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.s = x;
            x
        }
        fn next_01(&mut self) -> f32 {
            (self.next_u32() as f64 / 4294967296.0) as f32
        }
        fn next_11(&mut self) -> f32 {
            self.next_01() * 2.0 - 1.0
        }
        fn next_noise(&mut self) -> f32 {
            (self.next_11() + self.next_11() + self.next_11()) / 3.0
        }
    }
    pub struct PhonographFilter {
        frequency: f32,
        depth: f32,
        crackle: f32,
        flutter: f32,
        room: f32,
        mic_agc: f32,
        drive: f32,
        wow_lfo: Lfo,
        flutter_lfo: Lfo,
        drift: f32,
        delay: DelayLine,
        hp1_state: BiquadState,
        hp2_state: BiquadState,
        lp1_state: BiquadState,
        lp2_state: BiquadState,
        peak1_state: BiquadState,
        peak2_state: BiquadState,
        hiss_hp_state: BiquadState,
        hiss_lp_state: BiquadState,
        hp1_coeffs: BiquadCoeffs,
        hp2_coeffs: BiquadCoeffs,
        lp1_coeffs: BiquadCoeffs,
        lp2_coeffs: BiquadCoeffs,
        peak1_coeffs: BiquadCoeffs,
        peak2_coeffs: BiquadCoeffs,
        hiss_hp_coeffs: BiquadCoeffs,
        hiss_lp_coeffs: BiquadCoeffs,
        r1: DelayLine,
        r2: DelayLine,
        r3: DelayLine,
        room_damp: f32,
        tick_env: f32,
        tick_amp: f32,
        scratch_env: f32,
        scratch_amp: f32,
        env: f32,
        agc_gain: f32,
        rng: XorShift32,
    }
    impl PhonographFilter {
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            frequency: f32,
            depth: f32,
            crackle: f32,
            flutter: f32,
            room: f32,
            mic_agc: f32,
            drive: f32,
        ) -> Self {
            let mut filter = Self {
                frequency,
                depth,
                crackle,
                flutter,
                room,
                mic_agc,
                drive,
                wow_lfo: Lfo::new(),
                flutter_lfo: Lfo::new(),
                drift: 0.0,
                delay: DelayLine::new(BUFFER_SIZE),
                hp1_state: BiquadState::default(),
                hp2_state: BiquadState::default(),
                lp1_state: BiquadState::default(),
                lp2_state: BiquadState::default(),
                peak1_state: BiquadState::default(),
                peak2_state: BiquadState::default(),
                hiss_hp_state: BiquadState::default(),
                hiss_lp_state: BiquadState::default(),
                hp1_coeffs: BiquadCoeffs::default(),
                hp2_coeffs: BiquadCoeffs::default(),
                lp1_coeffs: BiquadCoeffs::default(),
                lp2_coeffs: BiquadCoeffs::default(),
                peak1_coeffs: BiquadCoeffs::default(),
                peak2_coeffs: BiquadCoeffs::default(),
                hiss_hp_coeffs: BiquadCoeffs::default(),
                hiss_lp_coeffs: BiquadCoeffs::default(),
                r1: DelayLine::new(PHONOGRAPH_R1_SIZE),
                r2: DelayLine::new(PHONOGRAPH_R2_SIZE),
                r3: DelayLine::new(PHONOGRAPH_R3_SIZE),
                room_damp: 0.0,
                tick_env: 0.0,
                tick_amp: 0.0,
                scratch_env: 0.0,
                scratch_amp: 0.0,
                env: 0.0,
                agc_gain: 1.0,
                rng: XorShift32::new(0x1337),
            };
            filter.recompute_filters();
            filter.update(frequency, depth, crackle, flutter, room, mic_agc, drive);
            filter
        }
        fn soft_clip(x: f32) -> f32 {
            let x2 = x * x;
            (x * (27.0 + x2)) / (27.0 + 9.0 * x2)
        }
        fn make_highpass(fc: f64, q: f64, fs: f64) -> BiquadCoeffs {
            let w0 = 2.0 * std::f64::consts::PI * (fc / fs);
            let cos_w0 = w0.cos();
            let sin_w0 = w0.sin();
            let alpha = sin_w0 / (2.0 * q);
            let a0 = 1.0 + alpha;
            BiquadCoeffs {
                b0: ((1.0 + cos_w0) / 2.0) / a0,
                b1: (-(1.0 + cos_w0)) / a0,
                b2: ((1.0 + cos_w0) / 2.0) / a0,
                a1: (-2.0 * cos_w0) / a0,
                a2: (1.0 - alpha) / a0,
            }
        }
        fn make_lowpass(fc: f64, q: f64, fs: f64) -> BiquadCoeffs {
            let w0 = 2.0 * std::f64::consts::PI * (fc / fs);
            let cos_w0 = w0.cos();
            let sin_w0 = w0.sin();
            let alpha = sin_w0 / (2.0 * q);
            let a0 = 1.0 + alpha;
            BiquadCoeffs {
                b0: ((1.0 - cos_w0) / 2.0) / a0,
                b1: (1.0 - cos_w0) / a0,
                b2: ((1.0 - cos_w0) / 2.0) / a0,
                a1: (-2.0 * cos_w0) / a0,
                a2: (1.0 - alpha) / a0,
            }
        }
        fn make_peaking(fc: f64, q: f64, gain_db: f64, fs: f64) -> BiquadCoeffs {
            let a = 10f64.powf(gain_db / 40.0);
            let w0 = 2.0 * std::f64::consts::PI * (fc / fs);
            let cos_w0 = w0.cos();
            let sin_w0 = w0.sin();
            let alpha = sin_w0 / (2.0 * q);
            let a0 = 1.0 + alpha / a;
            BiquadCoeffs {
                b0: (1.0 + alpha * a) / a0,
                b1: (-2.0 * cos_w0) / a0,
                b2: (1.0 - alpha * a) / a0,
                a1: (-2.0 * cos_w0) / a0,
                a2: (1.0 - alpha / a) / a0,
            }
        }
        #[allow(clippy::too_many_arguments)]
        pub fn update(
            &mut self,
            frequency: f32,
            depth: f32,
            crackle: f32,
            flutter: f32,
            room: f32,
            mic_agc: f32,
            drive: f32,
        ) {
            self.frequency = frequency.clamp(0.0, 1.0);
            self.depth = depth.clamp(0.0, 1.0);
            self.crackle = crackle.clamp(0.0, 1.0);
            self.flutter = flutter.clamp(0.0, 1.0);
            self.room = room.clamp(0.0, 1.0);
            self.mic_agc = mic_agc.clamp(0.0, 1.0);
            self.drive = drive.clamp(0.0, 1.0);
            self.wow_lfo.update(0.5, self.depth as f64);
            self.flutter_lfo.update(6.0, (self.flutter * 0.1) as f64);
        }
        fn recompute_filters(&mut self) {
            let fs = TARGET_SAMPLE_RATE as f64;
            let q = std::f64::consts::FRAC_1_SQRT_2;
            self.hp1_coeffs = Self::make_highpass(260.0, q, fs);
            self.hp2_coeffs = Self::make_highpass(260.0, q, fs);
            self.lp1_coeffs = Self::make_lowpass(3300.0, q, fs);
            self.lp2_coeffs = Self::make_lowpass(3300.0, q, fs);
            self.peak1_coeffs = Self::make_peaking(950.0, 1.1, 7.0, fs);
            self.peak2_coeffs = Self::make_peaking(2400.0, 1.6, 3.5, fs);
            self.hiss_hp_coeffs = Self::make_highpass(1800.0, q, fs);
            self.hiss_lp_coeffs = Self::make_lowpass(6500.0, q, fs);
        }
    }
    impl AudioFilter for PhonographFilter {
        fn process(&mut self, samples: &mut [i16]) {
            let fs = TARGET_SAMPLE_RATE as f32;
            let wow_max = self.depth * 0.014 * fs;
            let flutter_max = self.flutter * 0.0022 * fs;
            let center = 1.0 + wow_max + flutter_max;
            let drift_amount = self.depth * 0.0012 * fs;
            let drift_smooth = 0.00015;
            let hiss_gain = 0.01 * self.crackle;
            let tick_rate = 0.00002 * self.crackle;
            let scratch_rate = 0.0000025 * self.crackle;
            let d1 = 7.5 / 1000.0 * fs;
            let d2 = 12.0 / 1000.0 * fs;
            let d3 = 17.5 / 1000.0 * fs;
            let room_mix = 0.35 * self.room;
            let agc_on = self.mic_agc > 0.0;
            let target = 0.22;
            let atk = 0.006 + 0.01 * self.mic_agc;
            let rel = 0.0006 + 0.0012 * self.mic_agc;
            for chunk in samples.chunks_exact_mut(2) {
                let left_sample = chunk[0] as f32;
                let right_sample = chunk[1] as f32;
                let mut x = ((left_sample + right_sample) * 0.5) / 32768.0;
                let d_noise = self.rng.next_noise();
                self.drift += (d_noise * drift_amount - self.drift) * drift_smooth;
                let wow = self.wow_lfo.get_value() as f32;
                let flt = self.flutter_lfo.get_value() as f32;
                let mut dly = center + wow * wow_max + flt * flutter_max + self.drift;
                if dly < 1.0 {
                    dly = 1.0;
                }
                if dly > BUFFER_SIZE as f32 - 2.0 {
                    dly = BUFFER_SIZE as f32 - 2.0;
                }
                self.delay.write(x);
                x = self.delay.read(dly);
                if self.drive > 0.0 {
                    let g = 1.0 + self.drive * 6.0;
                    x = Self::soft_clip(x * g) / Self::soft_clip(g);
                }
                x = self.hp1_state.process(x as f64, &self.hp1_coeffs) as f32;
                x = self.hp2_state.process(x as f64, &self.hp2_coeffs) as f32;
                x = self.lp1_state.process(x as f64, &self.lp1_coeffs) as f32;
                x = self.lp2_state.process(x as f64, &self.lp2_coeffs) as f32;
                x = self.peak1_state.process(x as f64, &self.peak1_coeffs) as f32;
                x = self.peak2_state.process(x as f64, &self.peak2_coeffs) as f32;
                if self.crackle > 0.0 {
                    let mut n = self.rng.next_noise();
                    n = self.hiss_hp_state.process(n as f64, &self.hiss_hp_coeffs) as f32;
                    n = self.hiss_lp_state.process(n as f64, &self.hiss_lp_coeffs) as f32;
                    x += n * hiss_gain;
                    if self.rng.next_01() < tick_rate {
                        self.tick_env = 1.0;
                        self.tick_amp = self.rng.next_11() * (0.45 + self.crackle);
                    }
                    self.tick_env *= 0.965;
                    x += self.tick_amp * self.tick_env * 0.18;
                    if self.rng.next_01() < scratch_rate {
                        self.scratch_env = 1.0;
                        self.scratch_amp = self.rng.next_11() * (0.35 + self.crackle);
                    }
                    self.scratch_env *= 0.992;
                    x += self.scratch_amp * self.scratch_env * 0.06;
                }
                if self.room > 0.0 {
                    self.room_damp += 0.08 * (x - self.room_damp);
                    self.r1.write(self.room_damp);
                    self.r2.write(self.room_damp);
                    self.r3.write(self.room_damp);
                    let a = self.r1.read(d1);
                    let b = self.r2.read(d2);
                    let c = self.r3.read(d3);
                    x = x * (1.0 - room_mix) + (a + b + c) * (room_mix / 3.0);
                }
                if agc_on {
                    let ax = x.abs();
                    let coeff = if ax > self.env { atk } else { rel };
                    self.env += (ax - self.env) * coeff;
                    let desired = target / (self.env + 1e-6);
                    self.agc_gain += (desired - self.agc_gain) * 0.0015;
                    let g = self.agc_gain.clamp(0.35, 2.8);
                    x *= g;
                }
                let out = (x * 32768.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                chunk[0] = out;
                chunk[1] = out;
            }
        }
        fn is_enabled(&self) -> bool {
            self.depth > 0.0
                || self.crackle > 0.0
                || self.flutter > 0.0
                || self.room > 0.0
                || self.drive > 0.0
        }
        fn reset(&mut self) {
            self.delay.clear();
            self.r1.clear();
            self.r2.clear();
            self.r3.clear();
            self.wow_lfo.reset();
            self.flutter_lfo.reset();
            self.drift = 0.0;
            self.hp1_state.reset();
            self.hp2_state.reset();
            self.lp1_state.reset();
            self.lp2_state.reset();
            self.peak1_state.reset();
            self.peak2_state.reset();
            self.hiss_hp_state.reset();
            self.hiss_lp_state.reset();
            self.tick_env = 0.0;
            self.tick_amp = 0.0;
            self.scratch_env = 0.0;
            self.scratch_amp = 0.0;
            self.room_damp = 0.0;
            self.env = 0.0;
            self.agc_gain = 1.0;
        }
    }
}
pub mod reverb {
    use super::{AudioFilter, delay_line::DelayLine};
    use crate::audio::constants::TARGET_SAMPLE_RATE;
    const COMB_DELAYS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
    const ALLPASS_DELAYS: [usize; 4] = [556, 441, 341, 225];
    const STEREO_SPREAD: usize = 23;
    const SCALE_WET: f32 = 3.0;
    const SCALE_DRY: f32 = 2.0;
    const SCALE_DAMP: f32 = 0.4;
    const SCALE_ROOM: f32 = 0.28;
    const OFFSET_ROOM: f32 = 0.7;
    struct CombFilter {
        buffer: DelayLine,
        filter_store: f32,
        damp1: f32,
        damp2: f32,
        feedback: f32,
    }
    impl CombFilter {
        fn new(size: usize) -> Self {
            Self {
                buffer: DelayLine::new(size),
                filter_store: 0.0,
                damp1: 0.0,
                damp2: 0.0,
                feedback: 0.0,
            }
        }
        fn set_damp(&mut self, val: f32) {
            self.damp1 = val;
            self.damp2 = 1.0 - val;
        }
        fn set_feedback(&mut self, val: f32) {
            self.feedback = val;
        }
        fn process(&mut self, input: f32) -> f32 {
            let output = self.buffer.read(0.0);
            self.filter_store = output * self.damp2 + self.filter_store * self.damp1;
            let write_val = input + self.filter_store * self.feedback;
            self.buffer
                .write(write_val.clamp(i16::MIN as f32, i16::MAX as f32));
            output
        }
        fn clear(&mut self) {
            self.buffer.clear();
            self.filter_store = 0.0;
        }
    }
    pub struct ReverbFilter {
        comb_l: Vec<CombFilter>,
        comb_r: Vec<CombFilter>,
        allpass_l: Vec<DelayLine>,
        allpass_r: Vec<DelayLine>,
        allpass_coeff: f32,
        allpass_state_l: Vec<f32>,
        allpass_state_r: Vec<f32>,
        wet: f32,
        dry: f32,
        room_size: f32,
        damping: f32,
        width: f32,
    }
    impl ReverbFilter {
        pub fn new(mix: f32, room_size: f32, damping: f32, width: f32) -> Self {
            let fs = TARGET_SAMPLE_RATE as f64;
            let comb_l = COMB_DELAYS
                .iter()
                .map(|&d| CombFilter::new((d as f64 * fs / 44100.0) as usize))
                .collect();
            let comb_r = COMB_DELAYS
                .iter()
                .map(|&d| CombFilter::new(((d + STEREO_SPREAD) as f64 * fs / 44100.0) as usize))
                .collect();
            let allpass_l = ALLPASS_DELAYS
                .iter()
                .map(|&d| DelayLine::new((d as f64 * fs / 44100.0) as usize))
                .collect();
            let allpass_r = ALLPASS_DELAYS
                .iter()
                .map(|&d| DelayLine::new(((d + STEREO_SPREAD) as f64 * fs / 44100.0) as usize))
                .collect();
            let mut filter = Self {
                comb_l,
                comb_r,
                allpass_l,
                allpass_r,
                allpass_coeff: 0.5,
                allpass_state_l: vec![0.0; ALLPASS_DELAYS.len()],
                allpass_state_r: vec![0.0; ALLPASS_DELAYS.len()],
                wet: 0.0,
                dry: 1.0,
                room_size: 0.5,
                damping: 0.5,
                width: 1.0,
            };
            filter.update(mix, room_size, damping, width);
            filter
        }
        pub fn update(&mut self, mix: f32, room_size: f32, damping: f32, width: f32) {
            let mix = mix.clamp(0.0, 1.0);
            self.wet = mix * SCALE_WET;
            self.dry = (1.0 - mix) * SCALE_DRY;
            self.room_size = room_size.clamp(0.0, 1.0);
            let room_scaled = self.room_size * SCALE_ROOM + OFFSET_ROOM;
            self.damping = damping.clamp(0.0, 1.0);
            let damp_scaled = self.damping * SCALE_DAMP;
            self.width = width.clamp(0.0, 1.0);
            for comb in self.comb_l.iter_mut() {
                comb.set_feedback(room_scaled);
                comb.set_damp(damp_scaled);
            }
            for comb in self.comb_r.iter_mut() {
                comb.set_feedback(room_scaled);
                comb.set_damp(damp_scaled);
            }
        }
        fn process_allpass(
            input: f32,
            delay_line: &mut DelayLine,
            state_y1: &mut f32,
            coeff: f32,
        ) -> f32 {
            let delayed = delay_line.read(0.0);
            let output = -input + delayed + coeff * (input - *state_y1);
            delay_line.write(input.clamp(i16::MIN as f32, i16::MAX as f32));
            *state_y1 = output;
            output
        }
    }
    impl AudioFilter for ReverbFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.wet == 0.0 {
                return;
            }
            for chunk in samples.chunks_exact_mut(2) {
                let left_input = chunk[0] as f32;
                let right_input = chunk[1] as f32;
                let mono_input = (left_input + right_input) * 0.5;
                let mut left_out = 0.0;
                let mut right_out = 0.0;
                for j in 0..self.comb_l.len() {
                    left_out += self.comb_l[j].process(mono_input);
                    right_out += self.comb_r[j].process(mono_input);
                }
                for j in 0..self.allpass_l.len() {
                    left_out = Self::process_allpass(
                        left_out,
                        &mut self.allpass_l[j],
                        &mut self.allpass_state_l[j],
                        self.allpass_coeff,
                    );
                    right_out = Self::process_allpass(
                        right_out,
                        &mut self.allpass_r[j],
                        &mut self.allpass_state_r[j],
                        self.allpass_coeff,
                    );
                }
                let wet1 = self.wet * (self.width * 0.5 + 0.5);
                let wet2 = self.wet * ((1.0 - self.width) * 0.5);
                let final_left = left_input * self.dry + left_out * wet1 + right_out * wet2;
                let final_right = right_input * self.dry + right_out * wet1 + left_out * wet2;
                chunk[0] = final_left.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                chunk[1] = final_right.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.wet > 0.0
        }
        fn reset(&mut self) {
            for comb in self.comb_l.iter_mut() {
                comb.clear();
            }
            for comb in self.comb_r.iter_mut() {
                comb.clear();
            }
            for pass in self.allpass_l.iter_mut() {
                pass.clear();
            }
            for pass in self.allpass_r.iter_mut() {
                pass.clear();
            }
            for state in self.allpass_state_l.iter_mut() {
                *state = 0.0;
            }
            for state in self.allpass_state_r.iter_mut() {
                *state = 0.0;
            }
        }
    }
}
pub mod rotation {
    use super::{AudioFilter, lfo::Lfo};
    pub struct RotationFilter {
        lfo: Lfo,
    }
    impl RotationFilter {
        pub fn new(rotation_hz: f64) -> Self {
            let mut lfo = Lfo::new();
            lfo.update(rotation_hz, 1.0);
            Self { lfo }
        }
    }
    impl AudioFilter for RotationFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.lfo.frequency == 0.0 {
                return;
            }
            let num_frames = samples.len() / 2;
            for frame in 0..num_frames {
                let offset = frame * 2;
                let lfo_value = self.lfo.get_value();
                let left_factor = (1.0 - lfo_value) / 2.0;
                let right_factor = (1.0 + lfo_value) / 2.0;
                let left = samples[offset] as f64;
                let right = samples[offset + 1] as f64;
                let new_left = left * left_factor;
                let new_right = right * right_factor;
                samples[offset] = new_left.clamp(i16::MIN as f64, i16::MAX as f64) as i16;
                samples[offset + 1] = new_right.clamp(i16::MIN as f64, i16::MAX as f64) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.lfo.frequency != 0.0
        }
        fn reset(&mut self) {
            self.lfo.reset();
        }
    }
}
pub mod spatial {
    use super::{AudioFilter, delay_line::DelayLine, lfo::Lfo};
    use crate::audio::constants::TARGET_SAMPLE_RATE;
    const MAX_DELAY_MS: f32 = 30.0;
    const BUFFER_SIZE: usize = ((48000.0 * MAX_DELAY_MS) / 1000.0) as usize;
    pub struct SpatialFilter {
        depth: f32,
        rate: f32,
        left_delay: DelayLine,
        right_delay: DelayLine,
        lfo: Lfo,
    }
    impl SpatialFilter {
        pub fn new(rate: f32, depth: f32) -> Self {
            let mut filter = Self {
                depth: 0.0,
                rate: 0.0,
                left_delay: DelayLine::new(BUFFER_SIZE),
                right_delay: DelayLine::new(BUFFER_SIZE),
                lfo: Lfo::new(),
            };
            filter.update(rate, depth);
            filter
        }
        pub fn update(&mut self, rate: f32, depth: f32) {
            self.rate = rate;
            self.depth = depth.clamp(0.0, 1.0);
            self.lfo.update(self.rate as f64, 1.0);
        }
    }
    impl AudioFilter for SpatialFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.depth == 0.0 {
                return;
            }
            let fs = TARGET_SAMPLE_RATE as f32;
            let wet = self.depth * 0.5;
            let dry = 1.0 - wet;
            let feedback = -0.3;
            for chunk in samples.chunks_exact_mut(2) {
                let left_in = chunk[0] as f32;
                let right_in = chunk[1] as f32;
                let lfo_value = self.lfo.get_value() as f32;
                let delay_time_l = (5.0 + lfo_value * 2.0) * (fs / 1000.0);
                let delay_time_r = (5.0 - lfo_value * 2.0) * (fs / 1000.0);
                let delayed_left = self.left_delay.read(delay_time_l);
                let delayed_right = self.right_delay.read(delay_time_r);
                self.left_delay.write(
                    (left_in + delayed_left * feedback).clamp(i16::MIN as f32, i16::MAX as f32),
                );
                self.right_delay.write(
                    (right_in + delayed_right * feedback).clamp(i16::MIN as f32, i16::MAX as f32),
                );
                let new_left = left_in * dry + delayed_right * wet;
                let new_right = right_in * dry + delayed_left * wet;
                chunk[0] = new_left.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                chunk[1] = new_right.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.depth > 0.0
        }
        fn reset(&mut self) {
            self.left_delay.clear();
            self.right_delay.clear();
        }
    }
}
pub mod timescale {
    use super::AudioFilter;
    fn cubic_resample(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        0.5 * (2.0 * p1
            + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
    }
    pub struct TimescaleFilter {
        final_rate: f32,
        input_buffer: Vec<i16>,
        position: f32,
    }
    impl TimescaleFilter {
        pub fn new(speed: f64, pitch: f64, rate: f64) -> Self {
            let speed = speed.clamp(0.1, 5.0);
            let pitch = pitch.clamp(0.1, 5.0);
            let rate = rate.clamp(0.1, 5.0);
            let final_rate = (speed * pitch * rate) as f32;
            Self {
                final_rate,
                input_buffer: Vec::with_capacity(4096),
                position: 0.0,
            }
        }
        pub fn process_resample(&mut self, samples: &[i16]) -> Vec<i16> {
            if (self.final_rate - 1.0).abs() < f32::EPSILON {
                return samples.to_vec();
            }
            if self.final_rate <= 0.0 {
                return Vec::new();
            }
            self.input_buffer.extend_from_slice(samples);
            let num_input_samples = self.input_buffer.len();
            let num_input_frames = num_input_samples / 2;
            if num_input_frames < 4 {
                return Vec::new();
            }
            let output_frames_est = (num_input_frames as f32 / self.final_rate) as usize + 2;
            let mut output = Vec::with_capacity(output_frames_est * 2);
            while (self.position as usize) + 2 < num_input_frames {
                let i1 = self.position as usize;
                let frac = self.position - i1 as f32;
                let p0_idx = i1.saturating_sub(1);
                let p1_idx = i1;
                let p2_idx = i1 + 1;
                let p3_idx = i1 + 2;
                let p0_l = self.input_buffer[p0_idx * 2] as f32 / 32768.0;
                let p1_l = self.input_buffer[p1_idx * 2] as f32 / 32768.0;
                let p2_l = self.input_buffer[p2_idx * 2] as f32 / 32768.0;
                let p3_l = self.input_buffer[p3_idx * 2] as f32 / 32768.0;
                let out_l = cubic_resample(p0_l, p1_l, p2_l, p3_l, frac);
                output.push((out_l.clamp(-1.0, 1.0) * 32767.0) as i16);
                let p0_r = self.input_buffer[p0_idx * 2 + 1] as f32 / 32768.0;
                let p1_r = self.input_buffer[p1_idx * 2 + 1] as f32 / 32768.0;
                let p2_r = self.input_buffer[p2_idx * 2 + 1] as f32 / 32768.0;
                let p3_r = self.input_buffer[p3_idx * 2 + 1] as f32 / 32768.0;
                let out_r = cubic_resample(p0_r, p1_r, p2_r, p3_r, frac);
                output.push((out_r.clamp(-1.0, 1.0) * 32767.0) as i16);
                self.position += self.final_rate;
            }
            let consumed_frames = self.position.floor() as usize;
            let keep_from_frame = consumed_frames.saturating_sub(1);
            if keep_from_frame > 0 {
                let samples_to_drain = keep_from_frame * 2;
                if samples_to_drain < self.input_buffer.len() {
                    self.input_buffer.drain(0..samples_to_drain);
                    self.position -= keep_from_frame as f32;
                }
            }
            output
        }
    }
    impl AudioFilter for TimescaleFilter {
        fn process(&mut self, _samples: &mut [i16]) {}
        fn is_enabled(&self) -> bool {
            (self.final_rate - 1.0).abs() > f32::EPSILON
        }
        fn reset(&mut self) {
            self.input_buffer.clear();
            self.position = 0.0;
        }
    }
}
pub mod tremolo {
    use super::{AudioFilter, lfo::Lfo};
    pub struct TremoloFilter {
        lfo: Lfo,
    }
    impl TremoloFilter {
        pub fn new(frequency: f32, depth: f32) -> Self {
            let mut lfo = Lfo::new();
            let depth = depth.clamp(0.0, 1.0);
            lfo.update(frequency as f64, depth as f64);
            Self { lfo }
        }
    }
    impl AudioFilter for TremoloFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.lfo.depth == 0.0 || self.lfo.frequency == 0.0 {
                return;
            }
            for chunk in samples.chunks_exact_mut(2) {
                let multiplier = self.lfo.process();
                let left = (chunk[0] as f64 * multiplier) as i32;
                chunk[0] = left.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                let right = (chunk[1] as f64 * multiplier) as i32;
                chunk[1] = right.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.lfo.depth > 0.0 && self.lfo.frequency > 0.0
        }
        fn reset(&mut self) {
            self.lfo.reset();
        }
    }
}
pub mod vibrato {
    use super::{AudioFilter, delay_line::DelayLine, lfo::Lfo};
    use crate::audio::constants::TARGET_SAMPLE_RATE;
    const MAX_DELAY_MS: f64 = 20.0;
    pub struct VibratoFilter {
        lfo: Lfo,
        left_delay: DelayLine,
        right_delay: DelayLine,
    }
    impl VibratoFilter {
        pub fn new(frequency: f32, depth: f32) -> Self {
            let buffer_size = ((TARGET_SAMPLE_RATE as f64 * MAX_DELAY_MS) / 1000.0).ceil() as usize;
            let mut lfo = Lfo::new();
            let depth = depth.clamp(0.0, 2.0);
            lfo.update(frequency as f64, depth as f64);
            Self {
                lfo,
                left_delay: DelayLine::new(buffer_size),
                right_delay: DelayLine::new(buffer_size),
            }
        }
    }
    impl AudioFilter for VibratoFilter {
        fn process(&mut self, samples: &mut [i16]) {
            if self.lfo.depth == 0.0 || self.lfo.frequency == 0.0 {
                self.left_delay.clear();
                self.right_delay.clear();
                return;
            }
            let max_delay_width = self.lfo.depth * TARGET_SAMPLE_RATE as f64 * 0.005;
            let center_delay = max_delay_width;
            let num_frames = samples.len() / 2;
            for frame in 0..num_frames {
                let offset = frame * 2;
                let lfo_value = self.lfo.get_value();
                let delay = center_delay + lfo_value * max_delay_width;
                let left_sample = samples[offset] as f32;
                self.left_delay.write(left_sample);
                let delayed_left = self.left_delay.read(delay as f32);
                samples[offset] =
                    (delayed_left as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                let right_sample = samples[offset + 1] as f32;
                self.right_delay.write(right_sample);
                let delayed_right = self.right_delay.read(delay as f32);
                samples[offset + 1] =
                    (delayed_right as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            self.lfo.depth > 0.0 && self.lfo.frequency > 0.0
        }
        fn reset(&mut self) {
            self.lfo.reset();
            self.left_delay.clear();
            self.right_delay.clear();
        }
    }
}
pub mod volume {
    use super::AudioFilter;
    pub struct VolumeFilter {
        volume: f32,
    }
    impl VolumeFilter {
        pub fn new(volume: f32) -> Self {
            Self { volume }
        }
    }
    impl AudioFilter for VolumeFilter {
        fn process(&mut self, samples: &mut [i16]) {
            let vol = self.volume;
            for sample in samples.iter_mut() {
                *sample = (*sample as f32 * vol).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            }
        }
        fn is_enabled(&self) -> bool {
            (self.volume - 1.0).abs() > f32::EPSILON
        }
        fn reset(&mut self) {}
    }
}
use crate::{
    config::FiltersConfig,
    player::{EqBand, Filters},
};
pub fn validate_filters(filters: &Filters, config: &FiltersConfig) -> Vec<&'static str> {
    let mut invalid = Vec::new();
    let c = config;
    let f = filters;
    if f.volume.is_some() && !c.volume {
        invalid.push("volume");
    }
    if f.equalizer.is_some() && !c.equalizer {
        invalid.push("equalizer");
    }
    if f.karaoke.is_some() && !c.karaoke {
        invalid.push("karaoke");
    }
    if f.timescale.is_some() && !c.timescale {
        invalid.push("timescale");
    }
    if f.tremolo.is_some() && !c.tremolo {
        invalid.push("tremolo");
    }
    if f.vibrato.is_some() && !c.vibrato {
        invalid.push("vibrato");
    }
    if f.distortion.is_some() && !c.distortion {
        invalid.push("distortion");
    }
    if f.rotation.is_some() && !c.rotation {
        invalid.push("rotation");
    }
    if f.channel_mix.is_some() && !c.channel_mix {
        invalid.push("channelMix");
    }
    if f.low_pass.is_some() && !c.low_pass {
        invalid.push("lowPass");
    }
    if f.echo.is_some() && !c.echo {
        invalid.push("echo");
    }
    if f.high_pass.is_some() && !c.high_pass {
        invalid.push("highPass");
    }
    if f.normalization.is_some() && !c.normalization {
        invalid.push("normalization");
    }
    if f.chorus.is_some() && !c.chorus {
        invalid.push("chorus");
    }
    if f.compressor.is_some() && !c.compressor {
        invalid.push("compressor");
    }
    if f.flanger.is_some() && !c.flanger {
        invalid.push("flanger");
    }
    if f.phaser.is_some() && !c.phaser {
        invalid.push("phaser");
    }
    if f.phonograph.is_some() && !c.phonograph {
        invalid.push("phonograph");
    }
    if f.reverb.is_some() && !c.reverb {
        invalid.push("reverb");
    }
    if f.spatial.is_some() && !c.spatial {
        invalid.push("spatial");
    }
    invalid
}
pub trait AudioFilter: Send {
    fn process(&mut self, samples: &mut [i16]);
    fn is_enabled(&self) -> bool;
    fn reset(&mut self);
}
pub enum ConcreteFilter {
    Volume(volume::VolumeFilter),
    Equalizer(Box<equalizer::EqualizerFilter>),
    Karaoke(karaoke::KaraokeFilter),
    Tremolo(tremolo::TremoloFilter),
    Vibrato(vibrato::VibratoFilter),
    Rotation(rotation::RotationFilter),
    Distortion(distortion::DistortionFilter),
    ChannelMix(channel_mix::ChannelMixFilter),
    LowPass(low_pass::LowPassFilter),
    Echo(echo::EchoFilter),
    HighPass(high_pass::HighPassFilter),
    Normalization(normalization::NormalizationFilter),
    Chorus(chorus::ChorusFilter),
    Compressor(compressor::CompressorFilter),
    Flanger(flanger::FlangerFilter),
    Phaser(phaser::PhaserFilter),
    Phonograph(Box<phonograph::PhonographFilter>),
    Reverb(reverb::ReverbFilter),
    Spatial(spatial::SpatialFilter),
}
impl ConcreteFilter {
    #[inline(always)]
    pub fn process(&mut self, samples: &mut [i16]) {
        match self {
            Self::Volume(f) => f.process(samples),
            Self::Equalizer(f) => f.process(samples),
            Self::Karaoke(f) => f.process(samples),
            Self::Tremolo(f) => f.process(samples),
            Self::Vibrato(f) => f.process(samples),
            Self::Rotation(f) => f.process(samples),
            Self::Distortion(f) => f.process(samples),
            Self::ChannelMix(f) => f.process(samples),
            Self::LowPass(f) => f.process(samples),
            Self::Echo(f) => f.process(samples),
            Self::HighPass(f) => f.process(samples),
            Self::Normalization(f) => f.process(samples),
            Self::Chorus(f) => f.process(samples),
            Self::Compressor(f) => f.process(samples),
            Self::Flanger(f) => f.process(samples),
            Self::Phaser(f) => f.process(samples),
            Self::Phonograph(f) => f.process(samples),
            Self::Reverb(f) => f.process(samples),
            Self::Spatial(f) => f.process(samples),
        }
    }
    pub fn reset(&mut self) {
        match self {
            Self::Volume(f) => f.reset(),
            Self::Equalizer(f) => f.reset(),
            Self::Karaoke(f) => f.reset(),
            Self::Tremolo(f) => f.reset(),
            Self::Vibrato(f) => f.reset(),
            Self::Rotation(f) => f.reset(),
            Self::Distortion(f) => f.reset(),
            Self::ChannelMix(f) => f.reset(),
            Self::LowPass(f) => f.reset(),
            Self::Echo(f) => f.reset(),
            Self::HighPass(f) => f.reset(),
            Self::Normalization(f) => f.reset(),
            Self::Chorus(f) => f.reset(),
            Self::Compressor(f) => f.reset(),
            Self::Flanger(f) => f.reset(),
            Self::Phaser(f) => f.reset(),
            Self::Phonograph(f) => f.reset(),
            Self::Reverb(f) => f.reset(),
            Self::Spatial(f) => f.reset(),
        }
    }
}
pub struct FilterChain {
    filters: Vec<ConcreteFilter>,
    timescale: Option<timescale::TimescaleFilter>,
    timescale_buffer: Vec<i16>,
}
impl FilterChain {
    pub fn from_config(config: &Filters) -> Self {
        let mut filters = Vec::new();
        if let Some(vol) = config.volume {
            let f = volume::VolumeFilter::new(vol);
            if f.is_enabled() {
                filters.push(ConcreteFilter::Volume(f));
            }
        }
        if let Some(ref bands) = config.equalizer {
            let band_tuples: Vec<(u8, f32)> =
                bands.iter().map(|b: &EqBand| (b.band, b.gain)).collect();
            let f = equalizer::EqualizerFilter::new(&band_tuples);
            if f.is_enabled() {
                filters.push(ConcreteFilter::Equalizer(Box::new(f)));
            }
        }
        if let Some(ref k) = config.karaoke {
            let f = karaoke::KaraokeFilter::new(
                k.level.unwrap_or(1.0),
                k.mono_level.unwrap_or(1.0),
                k.filter_band.unwrap_or(220.0),
                k.filter_width.unwrap_or(100.0),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::Karaoke(f));
            }
        }
        if let Some(ref t) = config.tremolo {
            let f = tremolo::TremoloFilter::new(t.frequency.unwrap_or(2.0), t.depth.unwrap_or(0.5));
            if f.is_enabled() {
                filters.push(ConcreteFilter::Tremolo(f));
            }
        }
        if let Some(ref v) = config.vibrato {
            let f = vibrato::VibratoFilter::new(v.frequency.unwrap_or(2.0), v.depth.unwrap_or(0.5));
            if f.is_enabled() {
                filters.push(ConcreteFilter::Vibrato(f));
            }
        }
        if let Some(ref r) = config.rotation {
            let f = rotation::RotationFilter::new(r.rotation_hz.unwrap_or(0.0));
            if f.is_enabled() {
                filters.push(ConcreteFilter::Rotation(f));
            }
        }
        if let Some(ref d) = config.distortion {
            let f = distortion::DistortionFilter::new(
                d.sin_offset.unwrap_or(0.0),
                d.sin_scale.unwrap_or(1.0),
                d.cos_offset.unwrap_or(0.0),
                d.cos_scale.unwrap_or(1.0),
                d.tan_offset.unwrap_or(0.0),
                d.tan_scale.unwrap_or(1.0),
                d.offset.unwrap_or(0.0),
                d.scale.unwrap_or(1.0),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::Distortion(f));
            }
        }
        if let Some(ref cm) = config.channel_mix {
            let f = channel_mix::ChannelMixFilter::new(
                cm.left_to_left.unwrap_or(1.0),
                cm.left_to_right.unwrap_or(0.0),
                cm.right_to_left.unwrap_or(0.0),
                cm.right_to_right.unwrap_or(1.0),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::ChannelMix(f));
            }
        }
        if let Some(ref lp) = config.low_pass {
            let f = low_pass::LowPassFilter::new(lp.smoothing.unwrap_or(20.0));
            if f.is_enabled() {
                filters.push(ConcreteFilter::LowPass(f));
            }
        }
        if let Some(ref e) = config.echo {
            let f = echo::EchoFilter::new(e.echo_length.unwrap_or(1.0), e.decay.unwrap_or(0.5));
            if f.is_enabled() {
                filters.push(ConcreteFilter::Echo(f));
            }
        }
        if let Some(ref hp) = config.high_pass {
            let f = high_pass::HighPassFilter::new(
                hp.cutoff_frequency.unwrap_or(200),
                hp.boost_factor.unwrap_or(1.0),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::HighPass(f));
            }
        }
        if let Some(ref n) = config.normalization {
            let f = normalization::NormalizationFilter::new(
                n.max_amplitude.unwrap_or(1.0),
                n.adaptive.unwrap_or(true),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::Normalization(f));
            }
        }
        if let Some(ref c) = config.chorus {
            let f = chorus::ChorusFilter::new(
                c.rate.unwrap_or(1.5),
                c.depth.unwrap_or(1.0),
                c.delay.unwrap_or(2.0),
                c.mix.unwrap_or(0.5),
                c.feedback.unwrap_or(0.5),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::Chorus(f));
            }
        }
        if let Some(ref c) = config.compressor {
            let f = compressor::CompressorFilter::new(
                c.threshold.unwrap_or(-10.0),
                c.ratio.unwrap_or(2.0),
                c.attack.unwrap_or(5.0),
                c.release.unwrap_or(50.0),
                c.makeup_gain.unwrap_or(0.0),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::Compressor(f));
            }
        }
        if let Some(ref fl) = config.flanger {
            let f = flanger::FlangerFilter::new(
                fl.rate.unwrap_or(0.2),
                fl.depth.unwrap_or(1.0),
                fl.feedback.unwrap_or(0.5),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::Flanger(f));
            }
        }
        if let Some(ref p) = config.phaser {
            let f = phaser::PhaserFilter::new(
                p.stages.unwrap_or(4),
                p.rate.unwrap_or(0.0),
                p.depth.unwrap_or(1.0),
                p.feedback.unwrap_or(0.0),
                p.mix.unwrap_or(0.5),
                p.min_frequency.unwrap_or(100.0),
                p.max_frequency.unwrap_or(2500.0),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::Phaser(f));
            }
        }
        if let Some(ref ph) = config.phonograph {
            let f = phonograph::PhonographFilter::new(
                ph.frequency.unwrap_or(0.8),
                ph.depth.unwrap_or(0.25),
                ph.crackle.unwrap_or(0.18),
                ph.flutter.unwrap_or(0.18),
                ph.room.unwrap_or(0.22),
                ph.mic_agc.unwrap_or(0.25),
                ph.drive.unwrap_or(0.25),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::Phonograph(Box::new(f)));
            }
        }
        if let Some(ref r) = config.reverb {
            let f = reverb::ReverbFilter::new(
                r.mix.unwrap_or(0.0),
                r.room_size.unwrap_or(0.5),
                r.damping.unwrap_or(0.5),
                r.width.unwrap_or(1.0),
            );
            if f.is_enabled() {
                filters.push(ConcreteFilter::Reverb(f));
            }
        }
        if let Some(ref s) = config.spatial {
            let f = spatial::SpatialFilter::new(s.rate.unwrap_or(0.0), s.depth.unwrap_or(0.0));
            if f.is_enabled() {
                filters.push(ConcreteFilter::Spatial(f));
            }
        }
        let timescale = config.timescale.as_ref().and_then(|t| {
            let f = timescale::TimescaleFilter::new(
                t.speed.unwrap_or(1.0),
                t.pitch.unwrap_or(1.0),
                t.rate.unwrap_or(1.0),
            );
            if f.is_enabled() { Some(f) } else { None }
        });
        Self {
            filters,
            timescale,
            timescale_buffer: Vec::new(),
        }
    }
    pub fn is_active(&self) -> bool {
        !self.filters.is_empty() || self.timescale.is_some()
    }
    pub fn process(&mut self, samples: &mut [i16]) {
        for filter in self.filters.iter_mut() {
            filter.process(samples);
        }
        if let Some(ref mut ts) = self.timescale {
            let resampled = ts.process_resample(samples);
            self.timescale_buffer.extend_from_slice(&resampled);
            const MAX_TS_SAMPLES: usize = 1920 * 1024;
            if self.timescale_buffer.len() > MAX_TS_SAMPLES {
                let excess = self.timescale_buffer.len() - MAX_TS_SAMPLES;
                let excess = excess - (excess % 2);
                if excess > 0 {
                    self.timescale_buffer.drain(..excess);
                }
            }
        }
    }
    pub fn fill_frame(&mut self, output: &mut [i16]) -> bool {
        if self.timescale.is_none() {
            return false;
        }
        if self.timescale_buffer.len() >= output.len() {
            output.copy_from_slice(&self.timescale_buffer[..output.len()]);
            self.timescale_buffer.drain(..output.len());
            true
        } else {
            false
        }
    }
    pub fn has_timescale(&self) -> bool {
        self.timescale.is_some()
    }
    pub fn reset(&mut self) {
        for filter in self.filters.iter_mut() {
            filter.reset();
        }
        if let Some(ref mut ts) = self.timescale {
            ts.reset();
        }
        self.timescale_buffer.clear();
    }
}
