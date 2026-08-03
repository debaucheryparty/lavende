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
        read_buf: Vec<u8>,
    }
    impl MixLayer {
        pub fn new(id: String, rx: Receiver<PooledBuffer>, volume: f32) -> Self {
            let mut read_buf = crate::audio::buffer::get_byte_pool().acquire(1920 * 4);
            read_buf.resize(1920 * 4, 0);
            Self {
                id,
                rx,
                ring_buffer: RingBuffer::new(LAYER_BUFFER_SIZE),
                volume: volume.clamp(0.0, 1.0),
                fade: None,
                finished: false,
                read_buf,
            }
        }
        pub fn fill(&mut self) {
            while let Ok(pooled) = self.rx.try_recv() {
                let bytes = crate::audio::buffer::as_byte_slice(&pooled);
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
                .is_some_and(|f| f.is_finished() && f.target_vol == 0.0);
            fade_killed || (self.finished && self.ring_buffer.is_empty())
        }
        pub fn accumulate(&mut self, acc: &mut [i32]) {
            let byte_count = acc.len() * 2;
            if self.read_buf.len() < byte_count {
                self.read_buf.resize(byte_count, 0);
            }
            let read_len = self.ring_buffer.read_into(&mut self.read_buf[..byte_count]);
            if read_len == byte_count {
                let samples = crate::audio::buffer::as_i16_slice(&self.read_buf[..byte_count]);
                for (acc_val, &s) in acc.iter_mut().zip(samples.iter()) {
                    let mut current_vol = self.volume;
                    if let Some(fade) = &mut self.fade {
                        current_vol *= fade.current_vol(1);
                    }
                    *acc_val += (s as f32 * current_vol) as i32;
                }
            }
        }
    }
    impl Drop for MixLayer {
        fn drop(&mut self) {
            let buf = std::mem::take(&mut self.read_buf);
            crate::audio::buffer::get_byte_pool().release(buf);
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
            let mut acc_buf = Vec::with_capacity(1920);
            acc_buf.resize(1920, 0);
            Self {
                layers: HashMap::new(),
                max_layers: MAX_LAYERS,
                enabled: true,
                acc_buf,
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
            if self.acc_buf.len() < out_len {
                self.acc_buf.resize(out_len, 0);
            }
            let acc_slice = &mut self.acc_buf[..out_len];
            for (acc, &sample) in acc_slice.iter_mut().zip(main_frame.iter()) {
                *acc = sample as i32;
            }
            self.layers.retain(|_, layer| {
                layer.fill();
                !layer.is_dead()
            });
            for layer in self.layers.values_mut() {
                layer.accumulate(acc_slice);
            }
            for (out, &sum) in main_frame.iter_mut().zip(acc_slice.iter()) {
                *out = sum.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
    }
    pub struct Mixer {
        tracks: Vec<MixerTrack>,
        mix_buf: Vec<i32>,
        pub audio_mixer: AudioMixer,
        opus_passthrough_track: Option<usize>,
        pub stuck_detector: Arc<StuckDetector>,
    }
    pub struct FadeEnvelope {
        pub start_vol: f32,
        pub target_vol: f32,
        pub samples_total: usize,
        pub samples_passed: usize,
        step: f32,
        current: f32,
    }
    impl FadeEnvelope {
        pub fn new(start_vol: f32, target_vol: f32, duration_ms: u64, sample_rate: u32) -> Self {
            let samples_total = ((duration_ms as f64 / 1000.0) * sample_rate as f64) as usize;
            let step = if samples_total > 0 {
                (target_vol - start_vol) / samples_total as f32
            } else {
                0.0
            };
            Self {
                start_vol,
                target_vol,
                samples_total,
                samples_passed: 0,
                step,
                current: start_vol,
            }
        }
        pub fn current_vol(&mut self, advance: usize) -> f32 {
            if self.samples_passed >= self.samples_total || self.samples_total == 0 {
                return self.target_vol;
            }
            let val = self.current;
            self.samples_passed += advance;
            self.current += self.step * advance as f32;
            if self.step > 0.0 {
                self.current = self.current.min(self.target_vol);
            } else {
                self.current = self.current.max(self.target_vol);
            }
            val
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
            let mut mix_buf = Vec::with_capacity(1920);
            mix_buf.resize(1920, 0);
            Self {
                tracks: Vec::new(),
                mix_buf,
                audio_mixer: AudioMixer::new(),
                opus_passthrough_track: None,
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
                    .store(PlaybackState::Stopped as u8, Ordering::SeqCst);
            }
            self.tracks.clear();
            self.audio_mixer.layers.clear();
            self.audio_mixer.enabled = false;
            self.mix_buf.clear();
        }
        pub fn mix(&mut self, buf: &mut [i16]) -> bool {
            let out_len = buf.len();

            self.tracks
                .retain(|t| t.state.load(Ordering::Relaxed) != PlaybackState::Stopped as u8);

            let active_tracks: Vec<usize> = self
                .tracks
                .iter()
                .enumerate()
                .filter(|(_, t)| {
                    let state = PlaybackState::from(t.state.load(Ordering::Relaxed));
                    !matches!(state, PlaybackState::Paused | PlaybackState::Stopped)
                })
                .map(|(i, _)| i)
                .collect();

            if active_tracks.is_empty() {
                self.audio_mixer.mix(buf);
                return !self.audio_mixer.layers.is_empty();
            }

            if active_tracks.len() == 1 && self.audio_mixer.layers.is_empty() {
                let has_audio = self.process_single_track(active_tracks[0], buf);
                self.audio_mixer.mix(buf);
                return has_audio;
            }

            if self.mix_buf.len() < out_len {
                self.mix_buf.resize(out_len, 0);
            }
            unsafe {
                std::ptr::write_bytes(self.mix_buf.as_mut_ptr(), 0, out_len);
            }

            let mut has_audio = false;
            for &track_idx in &active_tracks {
                if self.process_track_to_mix(track_idx, out_len) {
                    has_audio = true;
                }
            }

            let mix_slice = &self.mix_buf[..out_len];
            for i in 0..out_len {
                buf[i] = mix_slice[i].clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }

            self.audio_mixer.mix(buf);
            if !self.audio_mixer.layers.is_empty() {
                has_audio = true;
            }
            has_audio
        }

        fn process_single_track(&mut self, track_idx: usize, buf: &mut [i16]) -> bool {
            let track = &mut self.tracks[track_idx];
            let state_raw = track.state.load(Ordering::Relaxed);
            let state = PlaybackState::from(state_raw);

            let vol_raw = track.volume.load(Ordering::Relaxed);
            let vol_f = f32::from_bits(vol_raw);

            let (fade_mult, should_remove_fade) = if let Some(fade) = &mut track.fade {
                let v = fade.current_vol(TARGET_SAMPLE_RATE as usize / (1000 / 20));
                if fade.is_finished() && fade.target_vol == 0.0 {
                    track
                        .state
                        .store(PlaybackState::Stopped as u8, Ordering::Relaxed);
                    buf.fill(0);
                    return false;
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

            let buf_len = buf.len();
            let mut filled = 0usize;

            if track.pending_pos < track.pending.len() {
                let n = buf_len.min(track.pending.len() - track.pending_pos);
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        track.pending.as_ptr().add(track.pending_pos),
                        buf.as_mut_ptr(),
                        n,
                    );
                }
                track.pending_pos += n;
                filled = n;
                if track.pending_pos >= track.pending.len() {
                    track.pending.clear();
                    track.pending_pos = 0;
                }
            }

            while filled < buf_len && !track.finished {
                match track.flow.try_pop_frame() {
                    Ok(Some(frame)) => {
                        let frame_len = frame.len();
                        let n = frame_len.min(buf_len - filled);
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                frame.as_ptr(),
                                buf.as_mut_ptr().add(filled),
                                n,
                            );
                        }
                        if n < frame_len {
                            track.pending.extend_from_slice(&frame[n..]);
                            track.pending_pos = 0;
                        }
                        filled += n;
                        crate::audio::buffer::release_buffer(frame);
                    }
                    Ok(None) => break,
                    Err(_) => {
                        track.finished = true;
                        break;
                    }
                }
            }

            if filled > 0 {
                track
                    .position
                    .fetch_add(filled as u64 / MIXER_CHANNELS as u64, Ordering::Relaxed);
                track.is_buffering.store(false, Ordering::Relaxed);
                self.stuck_detector.record_frame_received();
                if filled < buf_len {
                    unsafe {
                        std::ptr::write_bytes(buf.as_mut_ptr().add(filled), 0, buf_len - filled);
                    }
                }
                true
            } else {
                if !track.finished {
                    track.is_buffering.store(true, Ordering::Relaxed);
                }
                buf.fill(0);
                false
            }
        }

        fn process_track_to_mix(&mut self, track_idx: usize, out_len: usize) -> bool {
            let track = &mut self.tracks[track_idx];
            let state_raw = track.state.load(Ordering::Relaxed);
            let state = PlaybackState::from(state_raw);

            let vol_raw = track.volume.load(Ordering::Relaxed);
            let vol_f = f32::from_bits(vol_raw);

            let (fade_mult, should_remove_fade) = if let Some(fade) = &mut track.fade {
                let v = fade.current_vol(TARGET_SAMPLE_RATE as usize / (1000 / 20));
                if fade.is_finished() && fade.target_vol == 0.0 {
                    track
                        .state
                        .store(PlaybackState::Stopped as u8, Ordering::Relaxed);
                    return false;
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
                let n = out_len.min(track.pending.len() - track.pending_pos);
                let pending_slice = &track.pending[track.pending_pos..track.pending_pos + n];
                let mix_slice = &mut self.mix_buf[..n];
                for i in 0..n {
                    mix_slice[i] += pending_slice[i] as i32;
                }
                track.pending_pos += n;
                filled = n;
                if track.pending_pos >= track.pending.len() {
                    track.pending.clear();
                    track.pending_pos = 0;
                }
            }

            while filled < out_len && !track.finished {
                match track.flow.try_pop_frame() {
                    Ok(Some(frame)) => {
                        let frame_len = frame.len();
                        let n = frame_len.min(out_len - filled);
                        let frame_slice = &frame[..n];
                        let mix_slice = &mut self.mix_buf[filled..filled + n];
                        for i in 0..n {
                            mix_slice[i] += frame_slice[i] as i32;
                        }
                        if n < frame_len {
                            track.pending.extend_from_slice(&frame[n..]);
                            track.pending_pos = 0;
                        }
                        filled += n;
                        crate::audio::buffer::release_buffer(frame);
                    }
                    Ok(None) => break,
                    Err(_) => {
                        track.finished = true;
                        break;
                    }
                }
            }

            if filled > 0 {
                track
                    .position
                    .fetch_add(filled as u64 / MIXER_CHANNELS as u64, Ordering::Relaxed);
                track.is_buffering.store(false, Ordering::Relaxed);
                self.stuck_detector.record_frame_received();
                true
            } else {
                if !track.finished {
                    track.is_buffering.store(true, Ordering::Relaxed);
                }
                false
            }
        }
    }
}
pub use layer::MixLayer;
pub use mixer::{AudioMixer, Mixer};
