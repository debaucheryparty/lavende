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
        decoder_done: bool,
        frame_rx: Receiver<AudioFrame>,
        frame_tx: Option<Sender<AudioFrame>>,
        latest_opus: Option<Vec<u8>>,
        opus_decoder: audiopus::coder::Decoder,
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
                decoder_done: false,
                frame_rx,
                frame_tx,
                latest_opus: None,
                opus_decoder: audiopus::coder::Decoder::new(
                    audiopus::SampleRate::Hz48000,
                    audiopus::Channels::Stereo,
                )
                .expect("Failed to create Opus decoder"),
            }
        }
        pub fn run(&mut self) {
            while let Ok(frame_data) = self.frame_rx.recv() {
                match frame_data {
                    AudioFrame::Pcm(pooled) => {
                        if pooled.is_empty() {
                            self.pending_pcm.clear();
                            continue;
                        }
                        self.pending_pcm.extend_from_slice(&pooled);
                        while self.pending_pcm.len() >= FRAME_SIZE_SAMPLES {
                            let mut frame = acquire_buffer(FRAME_SIZE_SAMPLES);
                            frame.extend(self.pending_pcm.drain(..FRAME_SIZE_SAMPLES));
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
                while self.pending_pcm.len() < FRAME_SIZE_SAMPLES {
                    match self
                        .frame_rx
                        .recv_timeout(std::time::Duration::from_millis(5))
                    {
                        Ok(AudioFrame::Pcm(chunk)) if chunk.is_empty() => {
                            self.pending_pcm.clear();
                            self.decoder_done = false;
                        }
                        Ok(AudioFrame::Pcm(chunk)) => {
                            self.pending_pcm.extend_from_slice(&chunk);
                            crate::audio::buffer::release_buffer(chunk);
                        }
                        Ok(AudioFrame::Opus(packet)) => {
                            self.latest_opus = Some(packet.clone());
                            let mut pcm = vec![0i16; 1920 * 2];
                            let opus_packet =
                                audiopus::packet::Packet::try_from(packet.as_slice()).ok();
                            if let Ok(mut_signals) =
                                audiopus::MutSignals::try_from(pcm.as_mut_slice())
                            {
                                if let Ok(decoded_samples) =
                                    self.opus_decoder.decode(opus_packet, mut_signals, false)
                                {
                                    pcm.truncate(decoded_samples * 2);
                                    self.pending_pcm.extend_from_slice(&pcm);
                                }
                            }
                        }
                        Err(flume::RecvTimeoutError::Timeout) => break,
                        Err(flume::RecvTimeoutError::Disconnected) => {
                            self.decoder_done = true;
                            break;
                        }
                    }
                }
            }
            if self.pending_pcm.len() >= FRAME_SIZE_SAMPLES {
                let mut frame = acquire_buffer(FRAME_SIZE_SAMPLES);
                frame.extend(self.pending_pcm.drain(..FRAME_SIZE_SAMPLES));
                self.process_frame(&mut frame);
                Ok(Some(frame))
            } else if self.decoder_done {
                if !self.pending_pcm.is_empty() {
                    let mut frame = acquire_buffer(FRAME_SIZE_SAMPLES);
                    frame.extend(self.pending_pcm.drain(..));
                    frame.resize(FRAME_SIZE_SAMPLES, 0);
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
                            self.decoder_done = false;
                        } else {
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
