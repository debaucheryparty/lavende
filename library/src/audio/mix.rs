pub mod layer {
    use super::mixer::FadeEnvelope;
    use crate::audio::{RingBuffer, buffer::PooledBuffer, constants::LAYER_BUFFER_SIZE};
    use flume::Receiver;
    pub struct MixLayer {
        pub id: String,
        pub rx: Receiver<PooledBuffer>,
        pub ring_buffer: RingBuffer,
        pub volume: f32,
        pub fade: Option<FadeEnvelope>,
        pub finished: bool,
    }
    impl MixLayer {
        pub fn new(id: String, rx: Receiver<PooledBuffer>, volume: f32) -> Self {
            Self {
                id,
                rx,
                ring_buffer: RingBuffer::new(LAYER_BUFFER_SIZE),
                volume: volume.clamp(0.0, 1.0),
                fade: None,
                finished: false,
            }
        }
        pub fn fill(&mut self) {
            while let Ok(pooled) = self.rx.try_recv() {
                let bytes = unsafe {
                    std::slice::from_raw_parts(pooled.as_ptr() as *const u8, pooled.len() * 2)
                };
                self.ring_buffer.write(bytes);
                crate::audio::buffer::release_buffer(pooled);
            }
            if self.rx.is_disconnected() {
                self.finished = true;
            }
        }
        pub fn is_dead(&self) -> bool {
            let fade_killed = self
                .fade
                .as_ref()
                .map_or(false, |f| f.is_finished() && f.target_vol == 0.0);
            fade_killed || (self.finished && self.ring_buffer.is_empty())
        }
        pub fn accumulate(&mut self, acc: &mut [i32]) {
            let byte_count = acc.len() * 2;
            if let Some(bytes) = self.ring_buffer.read(byte_count) {
                let samples = unsafe {
                    std::slice::from_raw_parts(bytes.as_ptr() as *const i16, bytes.len() / 2)
                };
                for (acc_val, &s) in acc.iter_mut().zip(samples.iter()) {
                    let mut current_vol = self.volume;
                    if let Some(fade) = &mut self.fade {
                        current_vol *= fade.current_vol(1);
                    }
                    *acc_val += (s as f32 * current_vol).round() as i32;
                }
            }
        }
    }
}
pub mod mixer {
    use super::layer::MixLayer;
    use crate::{
        audio::{
            AudioFrame,
            buffer::PooledBuffer,
            constants::{MAX_LAYERS, MIXER_CHANNELS, TARGET_SAMPLE_RATE},
            flow::FlowController,
            playback::{StuckDetector, handle::PlaybackState},
        },
        config::player::PlayerConfig,
    };
    use flume::Receiver;
    use std::{
        collections::HashMap,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering},
        },
    };
    pub struct AudioMixer {
        pub layers: HashMap<String, MixLayer>,
        pub max_layers: usize,
        pub enabled: bool,
        acc_buf: Vec<i32>,
    }
    impl Default for AudioMixer {
        fn default() -> Self {
            Self::new()
        }
    }
    impl AudioMixer {
        pub fn new() -> Self {
            Self {
                layers: HashMap::new(),
                max_layers: MAX_LAYERS,
                enabled: true,
                acc_buf: Vec::with_capacity(1920),
            }
        }
        pub fn add_layer(
            &mut self,
            id: String,
            rx: Receiver<PooledBuffer>,
            volume: f32,
        ) -> Result<(), &'static str> {
            if self.layers.len() >= MAX_LAYERS {
                return Err("Maximum mix layers reached");
            }
            self.layers
                .insert(id.clone(), MixLayer::new(id, rx, volume));
            Ok(())
        }
        pub fn remove_layer(&mut self, id: &str) {
            self.layers.remove(id);
        }
        pub fn set_layer_volume(&mut self, id: &str, volume: f32) {
            if let Some(layer) = self.layers.get_mut(id) {
                layer.volume = volume.clamp(0.0, 1.0);
            }
        }
        pub fn mix(&mut self, main_frame: &mut [i16]) {
            if !self.enabled || self.layers.is_empty() {
                return;
            }
            let out_len = main_frame.len();
            if self.acc_buf.len() != out_len {
                self.acc_buf.resize(out_len, 0);
            }
            for (acc, &sample) in self.acc_buf.iter_mut().zip(main_frame.iter()) {
                *acc = sample as i32;
            }
            self.layers.retain(|_, layer| {
                layer.fill();
                !layer.is_dead()
            });
            for layer in self.layers.values_mut() {
                layer.accumulate(&mut self.acc_buf);
            }
            for (out, &sum) in main_frame.iter_mut().zip(self.acc_buf.iter()) {
                *out = sum.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
    }
    pub struct Mixer {
        tracks: Vec<MixerTrack>,
        mix_buf: Vec<i32>,
        pub audio_mixer: AudioMixer,
        opus_passthrough_track: Option<usize>,
        final_pcm_buf: Vec<i16>,
        pub stuck_detector: Arc<StuckDetector>,
    }
    pub struct FadeEnvelope {
        pub start_vol: f32,
        pub target_vol: f32,
        pub samples_total: usize,
        pub samples_passed: usize,
    }
    impl FadeEnvelope {
        pub fn new(start_vol: f32, target_vol: f32, duration_ms: u64, sample_rate: u32) -> Self {
            let samples_total = ((duration_ms as f64 / 1000.0) * sample_rate as f64) as usize;
            Self {
                start_vol,
                target_vol,
                samples_total,
                samples_passed: 0,
            }
        }
        pub fn current_vol(&mut self, advance: usize) -> f32 {
            if self.samples_passed >= self.samples_total || self.samples_total == 0 {
                return self.target_vol;
            }
            let progress = self.samples_passed as f32 / self.samples_total as f32;
            let current = self.start_vol + (self.target_vol - self.start_vol) * progress;
            self.samples_passed += advance;
            current
        }
        pub fn is_finished(&self) -> bool {
            self.samples_passed >= self.samples_total && self.samples_total > 0
        }
    }
    struct MixerTrack {
        flow: FlowController,
        pending: Vec<i16>,
        pending_pos: usize,
        state: Arc<AtomicU8>,
        volume: Arc<AtomicU32>,
        position: Arc<AtomicU64>,
        is_buffering: Arc<AtomicBool>,
        config: PlayerConfig,
        fade: Option<FadeEnvelope>,
        finished: bool,
    }
    impl Mixer {
        pub fn new(_sample_rate: u32) -> Self {
            Self {
                tracks: Vec::new(),
                mix_buf: Vec::with_capacity(1920),
                audio_mixer: AudioMixer::new(),
                opus_passthrough_track: None,
                final_pcm_buf: Vec::with_capacity(1920),
                stuck_detector: Arc::new(StuckDetector::new(10_000)),
            }
        }
        pub fn add_track(
            &mut self,
            rx: Receiver<AudioFrame>,
            state: Arc<AtomicU8>,
            volume: Arc<AtomicU32>,
            position: Arc<AtomicU64>,
            is_buffering: Arc<AtomicBool>,
            config: PlayerConfig,
        ) {
            let vol_raw = f32::from_bits(volume.load(Ordering::Acquire));
            let mut flow =
                FlowController::for_mixer(rx, TARGET_SAMPLE_RATE, MIXER_CHANNELS, vol_raw);
            flow.volume.set_volume_instant(vol_raw);

            let mut new_fade = None;
            if config.transitions.crossfade && config.transitions.crossfade_duration_ms > 0 {
                let duration = config.transitions.crossfade_duration_ms;
                for track in self.tracks.iter_mut() {
                    let cur_vol = track.fade.as_ref().map_or(1.0, |f| f.target_vol);
                    if cur_vol > 0.0 {
                        track.fade = Some(FadeEnvelope::new(
                            cur_vol,
                            0.0,
                            duration,
                            TARGET_SAMPLE_RATE,
                        ));
                    }
                }
                new_fade = Some(FadeEnvelope::new(0.0, 1.0, duration, TARGET_SAMPLE_RATE));
            }

            self.tracks.push(MixerTrack {
                flow,
                pending: Vec::new(),
                pending_pos: 0,
                state,
                volume,
                position,
                is_buffering,
                config,
                fade: new_fade,
                finished: false,
            });
        }
        pub fn set_passthrough_track(&mut self, track_index: usize) {
            self.opus_passthrough_track = Some(track_index);
        }
        pub fn take_opus_frame(&mut self) -> Option<Vec<u8>> {
            let active_count = self
                .tracks
                .iter()
                .filter(|t| {
                    let state = PlaybackState::from(t.state.load(Ordering::Acquire));
                    !matches!(
                        state,
                        PlaybackState::Paused
                            | PlaybackState::Stopped
                            | PlaybackState::Stopping
                            | PlaybackState::Starting
                    )
                })
                .count();

            if active_count != 1 {
                return None;
            }

            for track in self.tracks.iter_mut() {
                let state = PlaybackState::from(track.state.load(Ordering::Acquire));
                if matches!(
                    state,
                    PlaybackState::Paused
                        | PlaybackState::Stopped
                        | PlaybackState::Stopping
                        | PlaybackState::Starting
                ) {
                    continue;
                }

                let vol_f = f32::from_bits(track.volume.load(Ordering::Acquire));
                if (vol_f - 1.0).abs() > 0.001 || track.fade.is_some() {
                    return None;
                }

                if let Some(packet) = track.flow.take_opus() {
                    track.position.fetch_add(960, Ordering::Relaxed);
                    return Some(packet);
                }
            }
            None
        }
        pub fn stop_all(&mut self) {
            for track in self.tracks.iter_mut() {
                track
                    .state
                    .store(PlaybackState::Stopped as u8, Ordering::Release);
            }
            self.tracks.clear();
            self.audio_mixer.enabled = false;
        }
        pub fn mix(&mut self, buf: &mut [i16]) -> bool {
            let out_len = buf.len();
            if self.mix_buf.len() != out_len {
                self.mix_buf.resize(out_len, 0);
            }
            self.mix_buf.fill(0);
            self.tracks
                .retain(|t| t.state.load(Ordering::Acquire) != PlaybackState::Stopped as u8);
            let mut has_audio = false;
            for track in self.tracks.iter_mut() {
                let state = PlaybackState::from(track.state.load(Ordering::Acquire));
                if matches!(state, PlaybackState::Paused | PlaybackState::Stopped) {
                    continue;
                }
                let vol_f = f32::from_bits(track.volume.load(Ordering::Acquire));
                let (fade_mult, should_remove_fade) = if let Some(fade) = &mut track.fade {
                    let v = fade.current_vol(TARGET_SAMPLE_RATE as usize / (1000 / 20));
                    if fade.is_finished() && fade.target_vol == 0.0 {
                        track
                            .state
                            .store(PlaybackState::Stopped as u8, Ordering::Release);
                        continue;
                    }
                    (v, fade.is_finished())
                } else {
                    (1.0, false)
                };
                if should_remove_fade {
                    track.fade = None;
                }
                let effective_vol = vol_f * fade_mult;
                if (effective_vol - track.flow.volume.current_volume()).abs() > 0.001 {
                    track.flow.volume.set_volume_instant(effective_vol);
                }
                if state == PlaybackState::Stopping && !track.flow.tape.is_ramping() {
                    track.flow.tape.tape_to(
                        track.config.tape.tape_stop_duration_ms as f32,
                        false,
                        track.config.tape.curve,
                    );
                } else if state == PlaybackState::Starting && !track.flow.tape.is_ramping() {
                    track.flow.tape.tape_to(
                        track.config.tape.tape_stop_duration_ms as f32,
                        true,
                        track.config.tape.curve,
                    );
                }
                let mut filled = 0usize;
                if track.pending_pos < track.pending.len() {
                    let n = (out_len - filled).min(track.pending.len() - track.pending_pos);
                    for (acc, &s) in self.mix_buf[filled..filled + n]
                        .iter_mut()
                        .zip(&track.pending[track.pending_pos..track.pending_pos + n])
                    {
                        *acc += s as i32;
                    }
                    track.pending_pos += n;
                    filled += n;
                    if track.pending_pos >= track.pending.len() {
                        track.pending.clear();
                        track.pending_pos = 0;
                    }
                }
                'pull: while filled < out_len && !track.finished {
                    match track.flow.try_pop_frame() {
                        Ok(Some(frame)) => {
                            let n = frame.len().min(out_len - filled);
                            for (acc, &s) in
                                self.mix_buf[filled..filled + n].iter_mut().zip(&frame[..n])
                            {
                                *acc += s as i32;
                            }
                            if n < frame.len() {
                                track.pending.extend_from_slice(&frame[n..]);
                                track.pending_pos = 0;
                            }
                            filled += n;
                            crate::audio::buffer::release_buffer(frame);
                        }
                        Ok(None) => break 'pull,
                        Err(_) => {
                            track.finished = true;
                            break 'pull;
                        }
                    }
                }
                if filled > 0 {
                    has_audio = true;
                    track
                        .position
                        .fetch_add(filled as u64 / MIXER_CHANNELS as u64, Ordering::Relaxed);
                    track.is_buffering.store(false, Ordering::Release);
                    self.stuck_detector.record_frame_received();
                } else if !track.finished {
                    track.is_buffering.store(true, Ordering::Release);
                }
                if track.finished && track.pending.is_empty() && !track.flow.tape.is_active() {
                    track
                        .state
                        .store(PlaybackState::Stopped as u8, Ordering::Release);
                }
                if track.flow.tape.check_ramp_completed() {
                    match state {
                        PlaybackState::Stopping => {
                            track
                                .state
                                .store(PlaybackState::Paused as u8, Ordering::Release);
                        }
                        PlaybackState::Starting => {
                            track
                                .state
                                .store(PlaybackState::Playing as u8, Ordering::Release);
                        }
                        _ => {}
                    }
                }
            }
            if self.final_pcm_buf.len() != out_len {
                self.final_pcm_buf.resize(out_len, 0);
            }
            for (final_pcm, &sum) in self.final_pcm_buf.iter_mut().zip(self.mix_buf.iter()) {
                *final_pcm = sum.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
            self.audio_mixer.mix(&mut self.final_pcm_buf);
            if !self.audio_mixer.layers.is_empty() {
                has_audio = true;
            }
            buf.copy_from_slice(&self.final_pcm_buf);
            has_audio
        }
    }
}
pub use layer::MixLayer;
pub use mixer::{AudioMixer, Mixer};
