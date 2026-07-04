pub mod volume {
    use crate::audio::{
        constants::{INT16_MAX_F, INT16_MIN_F},
        effects::fade::FadeCurve,
    };
    pub struct VolumeEffect {
        current_volume: f32,
        target_volume: f32,
        start_volume: f32,
        fade_frames_total: usize,
        fade_frames_elapsed: usize,
        fade_active: bool,
        fade_curve: FadeCurve,
        limiter_softness: f32,
        threshold_value: f32,
        limit_headroom: f32,
        limiter_lut: [f32; 1024],
        channels: usize,
    }
    impl VolumeEffect {
        pub fn new(volume: f32, sample_rate: u32, channels: usize) -> Self {
            let limiter_threshold = 0.95_f32;
            let limiter_softness = 0.4_f32;
            let threshold_value = limiter_threshold * INT16_MAX_F;
            let limit_headroom = INT16_MAX_F - threshold_value;
            let mut limiter_lut = [0.0_f32; 1024];
            for (i, val) in limiter_lut.iter_mut().enumerate() {
                let overshoot = i as f32 / 1023.0 * 2.5;
                *val = 1.0 - (-overshoot * limiter_softness).exp();
            }
            let fade_frames_total = sample_rate as usize;
            Self {
                current_volume: volume,
                target_volume: volume,
                start_volume: volume,
                fade_frames_total,
                fade_frames_elapsed: fade_frames_total,
                fade_active: false,
                fade_curve: FadeCurve::Sinusoidal,
                limiter_softness,
                threshold_value,
                limit_headroom,
                limiter_lut,
                channels,
            }
        }
        pub fn set_volume(&mut self, volume: f32) {
            if (volume - self.target_volume).abs() < f32::EPSILON {
                return;
            }
            self.start_volume = self.current_volume;
            self.target_volume = volume;
            self.fade_frames_elapsed = 0;
            self.fade_active = self.fade_frames_total > 0;
            if !self.fade_active {
                self.current_volume = volume;
            }
        }
        pub fn set_volume_instant(&mut self, volume: f32) {
            self.current_volume = volume;
            self.target_volume = volume;
            self.start_volume = volume;
            self.fade_active = false;
            self.fade_frames_elapsed = self.fade_frames_total;
        }
        pub fn current_volume(&self) -> f32 {
            self.current_volume
        }
        #[inline(always)]
        fn apply_limiter(&self, value: f32) -> f32 {
            let abs = value.abs();
            if abs <= self.threshold_value || self.limit_headroom <= 0.0 {
                return value;
            }
            let overshoot_raw = (abs - self.threshold_value) / self.limit_headroom;
            let lut_idx = (overshoot_raw * 1023.0 / 2.5) as usize;
            let softened = if lut_idx < 1024 {
                self.limiter_lut[lut_idx]
            } else {
                1.0 - (-overshoot_raw * self.limiter_softness).exp()
            };
            let limited = self.threshold_value + self.limit_headroom * softened;
            value.signum() * limited.min(INT16_MAX_F)
        }
        pub fn process(&mut self, frame: &mut [i16]) {
            let sample_count = frame.len();
            if sample_count == 0 {
                return;
            }
            let (gain_start, gain_end) = if self.fade_active && self.fade_frames_total > 0 {
                let frames = sample_count / self.channels;
                let prev = self.fade_frames_elapsed;
                let next = (prev + frames).min(self.fade_frames_total);
                let t_start = prev as f32 / self.fade_frames_total as f32;
                let t_end = next as f32 / self.fade_frames_total as f32;
                let range = self.target_volume - self.start_volume;
                let gs = self.start_volume + range * self.fade_curve.value(t_start);
                let ge = self.start_volume + range * self.fade_curve.value(t_end);
                self.fade_frames_elapsed = next;
                if next >= self.fade_frames_total {
                    self.fade_active = false;
                    self.current_volume = self.target_volume;
                } else {
                    self.current_volume = ge;
                }
                (gs, ge)
            } else {
                let v = self.target_volume;
                (v, v)
            };
            if !self.fade_active && (gain_start - 1.0).abs() < 0.0001 {
                return;
            }
            let step = if sample_count > 1 {
                (gain_end - gain_start) / (sample_count - 1) as f32
            } else {
                0.0
            };
            let mut gain = gain_start;
            for s in frame.iter_mut() {
                let scaled = *s as f32 * gain;
                if scaled.abs() > self.threshold_value {
                    let limited = self.apply_limiter(scaled);
                    *s = limited.clamp(INT16_MIN_F, INT16_MAX_F) as i16;
                } else {
                    *s = scaled as i16;
                }
                gain += step;
            }
        }
    }
}
pub mod tape {
    use crate::config::player::TapeCurve;
    struct TapeState {
        start_rate: f32,
        target_rate: f32,
        duration_ms: f32,
        elapsed_ms: f32,
        curve: TapeCurve,
    }
    pub struct TapeEffect {
        sample_rate: u32,
        channels: usize,
        current_rate: f32,
        tape: Option<TapeState>,
        ramp_completed: bool,
        input_buffer: Vec<f32>,
        read_pos: f64,
    }
    impl TapeEffect {
        pub fn new(sample_rate: u32, channels: usize) -> Self {
            let max_size = (sample_rate as usize * channels * 10).max(96000);
            Self {
                sample_rate,
                channels,
                current_rate: 1.0,
                tape: None,
                ramp_completed: false,
                input_buffer: Vec::with_capacity(max_size),
                read_pos: 0.0,
            }
        }
        pub fn set_rate(&mut self, rate: f32) {
            self.current_rate = rate.clamp(0.01, 2.0);
            self.tape = None;
            self.ramp_completed = false;
        }
        pub fn tape_to(&mut self, duration_ms: f32, is_start: bool, curve_type: TapeCurve) {
            let target_rate = if is_start { 1.0 } else { 0.01 };
            if duration_ms <= 0.0 {
                self.current_rate = target_rate;
                self.tape = None;
                return;
            }
            self.tape = Some(TapeState {
                start_rate: self.current_rate,
                target_rate,
                duration_ms,
                elapsed_ms: 0.0,
                curve: curve_type,
            });
            self.ramp_completed = false;
        }
        pub fn is_active(&self) -> bool {
            self.tape.is_some() || (self.current_rate - 1.0).abs() > 0.001
        }
        pub fn is_ramping(&self) -> bool {
            self.tape.is_some()
        }
        pub fn check_ramp_completed(&mut self) -> bool {
            std::mem::replace(&mut self.ramp_completed, false)
        }
        pub fn process(&mut self, frame: &mut [i16]) {
            if frame.is_empty() || !self.is_active() {
                return;
            }
            let channels = self.channels;
            for &s in frame.iter() {
                self.input_buffer.push(s as f32 / 32767.0);
            }
            let mut out_idx = 0;
            let sample_duration_ms = 1000.0 / self.sample_rate as f32;
            while out_idx < frame.len() {
                if let Some(state) = &mut self.tape {
                    state.elapsed_ms += sample_duration_ms;
                    let t = (state.elapsed_ms / state.duration_ms).min(1.0);
                    let curve_t = state.curve.value(t);
                    self.current_rate =
                        state.start_rate + (state.target_rate - state.start_rate) * curve_t;
                    if t >= 1.0 {
                        self.current_rate = state.target_rate;
                        self.tape = None;
                        self.ramp_completed = true;
                    }
                }
                if self.current_rate <= 0.01 && self.tape.is_none() {
                    frame[out_idx..].fill(0);
                    break;
                }
                let i_pos = (self.read_pos.floor() as usize / channels) * channels;
                if i_pos + channels * 3 >= self.input_buffer.len() {
                    frame[out_idx..].fill(0);
                    break;
                }
                let frac = ((self.read_pos - i_pos as f64) / channels as f64) as f32;
                for c in 0..channels {
                    let p0 = if i_pos >= channels {
                        self.input_buffer[i_pos - channels + c]
                    } else {
                        self.input_buffer[i_pos + c]
                    };
                    let p1 = self.input_buffer[i_pos + c];
                    let p2 = self.input_buffer[i_pos + channels + c];
                    let p3 = self.input_buffer[i_pos + channels * 2 + c];
                    let val = 0.5
                        * (2.0 * p1
                            + (-p0 + p2) * frac
                            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * frac * frac
                            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * frac * frac * frac);
                    if out_idx < frame.len() {
                        frame[out_idx] = (val * 32767.0).clamp(-32768.0, 32767.0).round() as i16;
                        out_idx += 1;
                    }
                }
                self.read_pos += self.current_rate as f64 * channels as f64;
            }
            if self.read_pos > (self.sample_rate as f64 * channels as f64) {
                let integral = (self.read_pos.floor() as usize / channels) * channels;
                self.input_buffer.copy_within(integral.., 0);
                self.input_buffer
                    .truncate(self.input_buffer.len() - integral);
                self.read_pos -= integral as f64;
            }
        }
    }
}
pub mod fade {
    use crate::audio::constants::{INT16_MAX_F, INT16_MIN_F};
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum FadeCurve {
        Linear,
        Sinusoidal,
    }
    impl FadeCurve {
        pub fn value(self, t: f32) -> f32 {
            match self {
                FadeCurve::Linear => t,
                FadeCurve::Sinusoidal => 0.5 * (1.0 - (t * std::f32::consts::PI).cos()),
            }
        }
    }
    pub struct FadeEffect {
        current_gain: f32,
        target_gain: f32,
        start_gain: f32,
        fade_samples_total: usize,
        fade_samples_elapsed: usize,
        fade_active: bool,
        curve: FadeCurve,
    }
    impl FadeEffect {
        pub fn new(initial_gain: f32, _channels: usize) -> Self {
            Self {
                current_gain: initial_gain,
                target_gain: initial_gain,
                start_gain: initial_gain,
                fade_samples_total: 0,
                fade_samples_elapsed: 0,
                fade_active: false,
                curve: FadeCurve::Sinusoidal,
            }
        }
        pub fn set_gain(&mut self, gain: f32) {
            self.current_gain = gain;
            self.target_gain = gain;
            self.start_gain = gain;
            self.fade_active = false;
        }
        pub fn fade_to(
            &mut self,
            target: f32,
            duration_ms: u64,
            curve: FadeCurve,
            sample_rate: u32,
        ) {
            if duration_ms == 0 {
                self.set_gain(target);
                return;
            }
            self.start_gain = self.current_gain;
            self.target_gain = target;
            self.fade_samples_total = (sample_rate as u64 * duration_ms / 1000) as usize;
            self.fade_samples_elapsed = 0;
            self.fade_active = self.fade_samples_total > 0;
            self.curve = curve;
        }
        pub fn current_gain(&self) -> f32 {
            self.current_gain
        }
        pub fn is_done(&self) -> bool {
            !self.fade_active
        }
        pub fn process(&mut self, frame: &mut [i16]) {
            let sample_count = frame.len();
            if sample_count == 0 {
                return;
            }
            if !self.fade_active && (self.current_gain - 1.0).abs() < 1e-5 {
                return;
            }
            let (gain_start, gain_end) = if self.fade_active && self.fade_samples_total > 0 {
                let prev = self.fade_samples_elapsed;
                let next = (prev + sample_count).min(self.fade_samples_total);
                let t0 = prev as f32 / self.fade_samples_total as f32;
                let t1 = next as f32 / self.fade_samples_total as f32;
                let range = self.target_gain - self.start_gain;
                let gs = self.start_gain + range * self.curve.value(t0);
                let ge = self.start_gain + range * self.curve.value(t1);
                self.fade_samples_elapsed = next;
                if next >= self.fade_samples_total {
                    self.fade_active = false;
                    self.current_gain = self.target_gain;
                } else {
                    self.current_gain = ge;
                }
                (gs, ge)
            } else {
                let g = self.current_gain;
                (g, g)
            };
            let step = if sample_count > 1 {
                (gain_end - gain_start) / (sample_count - 1) as f32
            } else {
                0.0
            };
            let mut gain = gain_start;
            for s in frame.iter_mut() {
                let out = (*s as f32 * gain).clamp(INT16_MIN_F, INT16_MAX_F);
                *s = out.round() as i16;
                gain += step;
            }
        }
    }
}
pub mod crossfade {
    use super::fade::FadeCurve;
    use crate::audio::{
        RingBuffer,
        buffer::PooledBuffer,
        constants::{HALF_PI, INT16_MAX_F, INT16_MIN_F},
    };
    use flume::Receiver;
    pub struct CrossfadeController {
        sample_rate: u32,
        channels: usize,
        bytes_per_ms: usize,
        ring_buffer: Option<RingBuffer>,
        next_rx: Option<Receiver<PooledBuffer>>,
        active_fade: Option<CrossfadeState>,
        target_buffer_bytes: usize,
    }
    struct CrossfadeState {
        duration_ms: u64,
        elapsed_ms: f32,
        curve: FadeCurve,
    }
    impl CrossfadeController {
        pub fn new(sample_rate: u32, channels: usize) -> Self {
            let bytes_per_ms = (sample_rate as usize * channels * 2) / 1000;
            Self {
                sample_rate,
                channels,
                bytes_per_ms,
                ring_buffer: None,
                next_rx: None,
                active_fade: None,
                target_buffer_bytes: 0,
            }
        }
        pub fn prepare(&mut self, rx: Receiver<PooledBuffer>, duration_ms: u64) {
            self.clear();
            let buffer_size = (duration_ms as usize * self.bytes_per_ms).max(8192);
            self.ring_buffer = Some(RingBuffer::new(buffer_size));
            self.target_buffer_bytes = buffer_size;
            self.next_rx = Some(rx);
        }
        pub fn fill_buffer(&mut self) {
            let Some(rx) = &self.next_rx else { return };
            let Some(ring) = &mut self.ring_buffer else {
                return;
            };
            while let Ok(pooled) = rx.try_recv() {
                ring.write(crate::audio::buffer::as_byte_slice(&pooled));
            }
        }
        pub fn is_ready(&self) -> bool {
            let Some(ring) = &self.ring_buffer else {
                return false;
            };
            ring.len()
                >= (self.target_buffer_bytes * 8 / 10)
                    .min(self.sample_rate as usize * self.channels * 2)
        }
        pub fn start_crossfade(&mut self, duration_ms: u64, curve: FadeCurve) -> bool {
            if self.ring_buffer.is_none() || !self.is_ready() {
                return false;
            }
            self.active_fade = Some(CrossfadeState {
                duration_ms,
                elapsed_ms: 0.0,
                curve,
            });
            true
        }
        pub fn is_active(&self) -> bool {
            self.active_fade.is_some()
        }
        pub fn clear(&mut self) {
            self.ring_buffer = None;
            self.next_rx = None;
            self.active_fade = None;
            self.target_buffer_bytes = 0;
        }
        pub fn process(&mut self, frame: &mut [i16]) -> bool {
            let (elapsed, duration, curve) = match &self.active_fade {
                Some(s) => (s.elapsed_ms, s.duration_ms as f32, s.curve),
                None => return false,
            };
            let sample_count = frame.len();
            let byte_count = sample_count * 2;
            let next_bytes = if let Some(ring) = &mut self.ring_buffer {
                ring.read(byte_count)
            } else {
                return false;
            };
            let Some(next_bytes) = next_bytes else {
                return false;
            };
            let next_samples_raw = crate::audio::buffer::as_i16_slice(&next_bytes);
            let chunk_ms =
                (sample_count as f32 / self.channels as f32 / self.sample_rate as f32) * 1000.0;
            let t_start = (elapsed / duration).min(1.0);
            let t_end = ((elapsed + chunk_ms) / duration).min(1.0);
            let (out_start, in_start) = fade_gains(t_start, curve);
            let (out_end, in_end) = fade_gains(t_end, curve);
            let step_out = if sample_count > 1 {
                (out_end - out_start) / (sample_count - 1) as f32
            } else {
                0.0
            };
            let step_in = if sample_count > 1 {
                (in_end - in_start) / (sample_count - 1) as f32
            } else {
                0.0
            };
            let mut g_out = out_start;
            let mut g_in = in_start;
            for (sample, &next_val) in frame.iter_mut().zip(next_samples_raw.iter()) {
                let mixed = (*sample as f32 * g_out) + (next_val as f32 * g_in);
                *sample = mixed.clamp(INT16_MIN_F, INT16_MAX_F) as i16;
                g_out += step_out;
                g_in += step_in;
            }
            let state = self.active_fade.as_mut().unwrap();
            state.elapsed_ms += chunk_ms;
            let finished = state.elapsed_ms >= state.duration_ms as f32;
            if finished {
                self.active_fade = None;
            }
            finished
        }
    }
    fn fade_gains(t: f32, curve: FadeCurve) -> (f32, f32) {
        let t = t.clamp(0.0, 1.0);
        match curve {
            FadeCurve::Linear => (1.0 - t, t),
            FadeCurve::Sinusoidal => ((t * HALF_PI).cos(), (t * HALF_PI).sin()),
        }
    }
}
use crate::audio::buffer::PooledBuffer;
use std::sync::atomic::{AtomicU8, AtomicU64};
pub struct ProcessContext<'a> {
    pub mix_buf: &'a mut [i32],
    pub i: &'a mut usize,
    pub out_len: usize,
    pub vol: f32,
    pub stash: &'a mut Vec<i16>,
    pub rx: &'a flume::Receiver<PooledBuffer>,
    pub state_atomic: &'a AtomicU8,
    pub position_atomic: &'a AtomicU64,
}
pub trait TransitionEffect: Send {
    fn process(&mut self, ctx: ProcessContext<'_>) -> bool;
}
