pub mod controller {
    use crate::audio::{
        AudioFrame,
        buffer::{PooledBuffer, acquire_buffer},
        constants::FRAME_SIZE_SAMPLES,
        effects::{
            crossfade::CrossfadeController, fade::FadeEffect, tape::TapeEffect,
            volume::VolumeEffect,
        },
        error::AudioError,
    };
    use flume::{Receiver, Sender};
    pub struct FlowController {
        pub tape: TapeEffect,
        pub volume: VolumeEffect,
        pub fade: FadeEffect,
        pub crossfade: CrossfadeController,
        pending_pcm: Vec<i16>,
        pending_pcm_pos: usize,
        decoder_done: bool,
        frame_rx: Receiver<AudioFrame>,
        frame_tx: Option<Sender<AudioFrame>>,
        latest_opus: Option<Vec<u8>>,
        opus_decoder: Option<audiopus::coder::Decoder>,
        opus_pcm: Vec<i16>,
    }
    impl FlowController {
        pub fn new(
            frame_rx: Receiver<AudioFrame>,
            frame_tx: Sender<AudioFrame>,
            sample_rate: u32,
            channels: usize,
            volume: f32,
        ) -> Self {
            Self::build(frame_rx, Some(frame_tx), sample_rate, channels, volume)
        }
        pub fn for_mixer(
            frame_rx: Receiver<AudioFrame>,
            sample_rate: u32,
            channels: usize,
            volume: f32,
        ) -> Self {
            Self::build(frame_rx, None, sample_rate, channels, volume)
        }
        fn build(
            frame_rx: Receiver<AudioFrame>,
            frame_tx: Option<Sender<AudioFrame>>,
            sample_rate: u32,
            channels: usize,
            volume: f32,
        ) -> Self {
            Self {
                tape: TapeEffect::new(sample_rate, channels),
                volume: VolumeEffect::new(volume, sample_rate, channels),
                fade: FadeEffect::new(1.0, channels),
                crossfade: CrossfadeController::new(sample_rate, channels),
                pending_pcm: Vec::with_capacity(FRAME_SIZE_SAMPLES * 2),
                pending_pcm_pos: 0,
                decoder_done: false,
                frame_rx,
                frame_tx,
                latest_opus: None,
                opus_decoder: None,
                opus_pcm: vec![0i16; 1920 * 2],
            }
        }
        #[inline]
        fn pending_len(&self) -> usize {
            self.pending_pcm.len() - self.pending_pcm_pos
        }
        #[inline]
        fn compact_pending(&mut self) {
            if self.pending_pcm_pos > 0 {
                self.pending_pcm.copy_within(self.pending_pcm_pos.., 0);
                self.pending_pcm.truncate(self.pending_len());
                self.pending_pcm_pos = 0;
            }
        }
        fn take_frame(&mut self) -> PooledBuffer {
            let end = self.pending_pcm_pos + FRAME_SIZE_SAMPLES;
            let mut frame = acquire_buffer(FRAME_SIZE_SAMPLES);
            frame.extend_from_slice(&self.pending_pcm[self.pending_pcm_pos..end]);
            self.pending_pcm_pos = end;
            if self.pending_pcm_pos > self.pending_pcm.len() / 2 {
                self.compact_pending();
            }
            frame
        }
        pub fn run(&mut self) {
            while let Ok(frame_data) = self.frame_rx.recv() {
                match frame_data {
                    AudioFrame::Pcm(pooled) => {
                        if pooled.is_empty() {
                            self.pending_pcm.clear();
                            self.pending_pcm_pos = 0;
                            continue;
                        }
                        self.compact_pending();
                        self.pending_pcm.extend_from_slice(&pooled);
                        while self.pending_len() >= FRAME_SIZE_SAMPLES {
                            let mut frame = self.take_frame();
                            self.process_frame(&mut frame);
                            if self
                                .frame_tx
                                .as_ref()
                                .is_some_and(|tx| tx.send(AudioFrame::Pcm(frame)).is_err())
                            {
                                return;
                            }
                        }
                    }
                    AudioFrame::Opus(packet) => {
                        if let Some(tx) = &self.frame_tx
                            && tx.send(AudioFrame::Opus(packet)).is_err()
                        {
                            return;
                        }
                    }
                }
            }
        }
        pub fn try_pop_frame(&mut self) -> Result<Option<PooledBuffer>, AudioError> {
            if !self.decoder_done {
                while self.pending_len() < FRAME_SIZE_SAMPLES {
                    match self.frame_rx.try_recv() {
                        Ok(AudioFrame::Pcm(chunk)) if chunk.is_empty() => {
                            self.pending_pcm.clear();
                            self.pending_pcm_pos = 0;
                            self.decoder_done = false;
                        }
                        Ok(AudioFrame::Pcm(chunk)) => {
                            self.compact_pending();
                            self.pending_pcm.extend_from_slice(&chunk);
                            crate::audio::buffer::release_buffer(chunk);
                        }
                        Ok(AudioFrame::Opus(packet)) => {
                            let opus_packet =
                                audiopus::packet::Packet::try_from(packet.as_slice()).ok();
                            if let Ok(mut_signals) =
                                audiopus::MutSignals::try_from(self.opus_pcm.as_mut_slice())
                            {
                                let decoder = self.opus_decoder.get_or_insert_with(|| {
                                    audiopus::coder::Decoder::new(
                                        audiopus::SampleRate::Hz48000,
                                        audiopus::Channels::Stereo,
                                    )
                                    .expect("Failed to create Opus decoder")
                                });
                                if let Ok(decoded_samples) =
                                    decoder.decode(opus_packet, mut_signals, false)
                                {
                                    self.compact_pending();
                                    self.pending_pcm
                                        .extend_from_slice(&self.opus_pcm[..decoded_samples * 2]);
                                }
                            }
                            self.latest_opus = Some(packet);
                        }
                        Err(flume::TryRecvError::Empty) => break,
                        Err(flume::TryRecvError::Disconnected) => {
                            self.decoder_done = true;
                            break;
                        }
                    }
                }
            }
            if self.pending_len() >= FRAME_SIZE_SAMPLES {
                let mut frame = self.take_frame();
                self.process_frame(&mut frame);
                Ok(Some(frame))
            } else if self.decoder_done {
                if self.pending_len() > 0 {
                    self.compact_pending();
                    let remaining = self.pending_pcm.len();
                    let mut frame = acquire_buffer(FRAME_SIZE_SAMPLES);
                    frame.extend_from_slice(&self.pending_pcm[..remaining]);
                    frame.resize(FRAME_SIZE_SAMPLES, 0);
                    self.pending_pcm.clear();
                    self.pending_pcm_pos = 0;
                    self.process_frame(&mut frame);
                    Ok(Some(frame))
                } else {
                    Err(AudioError::DecoderFinished)
                }
            } else {
                Ok(None)
            }
        }
        pub fn process_frame(&mut self, frame: &mut [i16]) {
            self.tape.process(frame);
            self.volume.process(frame);
            self.fade.process(frame);
            self.crossfade.fill_buffer();
            if self.crossfade.is_active() {
                self.crossfade.process(frame);
            }
        }
        pub fn take_opus(&mut self) -> Option<Vec<u8>> {
            if let Some(packet) = self.latest_opus.take() {
                return Some(packet);
            }
            while let Ok(frame) = self.frame_rx.try_recv() {
                match frame {
                    AudioFrame::Opus(packet) => return Some(packet),
                    AudioFrame::Pcm(chunk) => {
                        if chunk.is_empty() {
                            self.pending_pcm.clear();
                            self.pending_pcm_pos = 0;
                            self.decoder_done = false;
                        } else {
                            self.compact_pending();
                            self.pending_pcm.extend_from_slice(&chunk);
                            crate::audio::buffer::release_buffer(chunk);
                        }
                    }
                }
            }
            None
        }
    }
}
pub use controller::FlowController;
