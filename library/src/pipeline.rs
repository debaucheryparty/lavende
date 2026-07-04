pub mod decoder {
    use super::resample::LinearResampler;
    use std::io::ErrorKind;
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::{CODEC_TYPE_NULL, Decoder, DecoderOptions};
    use symphonia::core::errors::Error;
    use symphonia::core::formats::{FormatOptions, FormatReader};
    use symphonia::core::io::{MediaSource, MediaSourceStream};
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    pub struct AudioDecoder {
        format: Box<dyn FormatReader>,
        decoder: Box<dyn Decoder>,
        track_id: u32,
        resampler: LinearResampler,
        sample_buf: Option<SampleBuffer<i16>>,
        raw_buffer: Vec<i16>,
        downmix_buf: Vec<i16>,
    }
    impl AudioDecoder {
        pub fn new(source: Box<dyn MediaSource>, ext_hint: Option<&str>) -> Result<Self, String> {
            let mss = MediaSourceStream::new(source, Default::default());
            let mut hint = Hint::new();
            if let Some(ext) = ext_hint {
                hint.with_extension(ext);
            }
            let probed = symphonia::default::get_probe()
                .format(
                    &hint,
                    mss,
                    &FormatOptions::default(),
                    &MetadataOptions::default(),
                )
                .map_err(|e| format!("Failed to probe format: {e}"))?;
            let format = probed.format;
            let track = format
                .tracks()
                .iter()
                .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
                .ok_or_else(|| "No audio track found in media source".to_string())?;
            let track_id = track.id;
            let sample_rate = track.codec_params.sample_rate.unwrap_or(48000);
            let decoder = symphonia::default::get_codecs()
                .make(&track.codec_params, &DecoderOptions::default())
                .map_err(|e| format!("Failed to create decoder: {e}"))?;
            let resampler = LinearResampler::new(sample_rate, 48000, 2);
            Ok(Self {
                format,
                decoder,
                track_id,
                resampler,
                sample_buf: None,
                raw_buffer: Vec::with_capacity(48000 * 2),
                downmix_buf: Vec::new(),
            })
        }
        pub fn read_frame(&mut self) -> Result<Option<Vec<i16>>, String> {
            const TARGET_FRAME_SAMPLES: usize = 960 * 2;
            while self.raw_buffer.len() < TARGET_FRAME_SAMPLES {
                let packet = match self.format.next_packet() {
                    Ok(p) => p,
                    Err(Error::IoError(ref e)) if e.kind() == ErrorKind::UnexpectedEof => {
                        break;
                    }
                    Err(e) => return Err(format!("Format read error: {e}")),
                };
                if packet.track_id() != self.track_id {
                    continue;
                }
                match self.decoder.decode(&packet) {
                    Ok(decoded) => {
                        let spec = *decoded.spec();
                        let mut buf = self.sample_buf.take().unwrap_or_else(|| {
                            SampleBuffer::<i16>::new(decoded.capacity() as u64, spec)
                        });
                        buf.copy_interleaved_ref(decoded);
                        let samples = buf.samples();
                        if !samples.is_empty() {
                            let frame_channels = spec.channels.count();
                            let stereo_pcm = if frame_channels == 2 {
                                samples
                            } else {
                                self.downmix_buf.clear();
                                let num_frames = samples.len() / frame_channels;
                                for i in 0..num_frames {
                                    let frame =
                                        &samples[i * frame_channels..(i + 1) * frame_channels];
                                    let mut l = 0i32;
                                    let mut r = 0i32;
                                    for (ch, &sample) in frame.iter().enumerate() {
                                        if ch % 2 == 0 {
                                            l += sample as i32;
                                        } else {
                                            r += sample as i32;
                                        }
                                    }
                                    let left_count = frame_channels.div_ceil(2);
                                    let right_count = frame_channels / 2;
                                    self.downmix_buf.push((l / left_count as i32) as i16);
                                    if right_count > 0 {
                                        self.downmix_buf.push((r / right_count as i32) as i16);
                                    } else {
                                        self.downmix_buf.push((l / left_count as i32) as i16);
                                    }
                                }
                                &self.downmix_buf[..]
                            };
                            if self.resampler.is_passthrough() {
                                self.raw_buffer.extend_from_slice(stereo_pcm);
                            } else {
                                self.resampler.process(stereo_pcm, &mut self.raw_buffer);
                            }
                        }
                        self.sample_buf = Some(buf);
                    }
                    Err(Error::IoError(ref e)) if e.kind() == ErrorKind::UnexpectedEof => {
                        break;
                    }
                    Err(Error::ResetRequired) => {
                        self.decoder.reset();
                        self.resampler.reset();
                        self.sample_buf = None;
                    }
                    Err(e) => return Err(format!("Decode error: {e}")),
                }
            }
            if self.raw_buffer.len() >= TARGET_FRAME_SAMPLES {
                let frame = self.raw_buffer.drain(0..TARGET_FRAME_SAMPLES).collect();
                Ok(Some(frame))
            } else if !self.raw_buffer.is_empty() {
                let mut frame = std::mem::take(&mut self.raw_buffer);
                frame.resize(TARGET_FRAME_SAMPLES, 0);
                Ok(Some(frame))
            } else {
                Ok(None)
            }
        }
        pub fn seek(&mut self, position_ms: u64) -> Result<(), String> {
            use symphonia::core::formats::{SeekMode, SeekTo};
            use symphonia::core::units::Time;
            let time = Time::from(position_ms as f64 / 1000.0);
            self.format
                .seek(
                    SeekMode::Coarse,
                    SeekTo::Time {
                        time,
                        track_id: Some(self.track_id),
                    },
                )
                .map_err(|e| format!("Symphonia seek error: {e}"))?;
            self.decoder.reset();
            self.resampler.reset();
            self.sample_buf = None;
            self.raw_buffer.clear();
            Ok(())
        }
    }
}
pub mod encoder {
    use audiopus::{Application, Bitrate, Channels, SampleRate, coder::Encoder as OpusEncoder};
    pub struct AudioEncoder {
        encoder: OpusEncoder,
    }
    impl AudioEncoder {
        pub fn new() -> Result<Self, String> {
            let mut encoder =
                OpusEncoder::new(SampleRate::Hz48000, Channels::Stereo, Application::Audio)
                    .map_err(|e| format!("Failed to create Opus encoder: {e:?}"))?;
            encoder
                .set_bitrate(Bitrate::Auto)
                .map_err(|e| format!("Failed to set Opus bitrate: {e:?}"))?;
            Ok(Self { encoder })
        }
        pub fn encode(&mut self, pcm: &[i16], out: &mut [u8]) -> Result<usize, String> {
            self.encoder
                .encode(pcm, out)
                .map_err(|e| format!("Opus encoding failed: {e:?}"))
        }
    }
}
pub mod resample {
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
        pub fn process(&mut self, input: &[i16], output: &mut Vec<i16>) {
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
