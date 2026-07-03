pub mod format {
use crate::common::types::AudioFormat;
pub fn detect_format(header: &[u8]) -> AudioFormat {
    if header.len() < 4 {
        return AudioFormat::Unknown;
    }
    if header.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return AudioFormat::Webm;
    }
    if header.len() >= 8 && &header[4..8] == b"ftyp" {
        return AudioFormat::Mp4;
    }
    if header.starts_with(b"OggS") {
        return AudioFormat::Ogg;
    }
    if header.starts_with(b"fLaC") {
        return AudioFormat::Flac;
    }
    if header.starts_with(b"RIFF") && header.len() >= 12 && &header[8..12] == b"WAVE" {
        return AudioFormat::Wav;
    }
    if header.starts_with(b"ID3") {
        return AudioFormat::Mp3;
    }
    if header[0] == 0xFF {
        let b1 = header[1];
        let is_sync = (b1 & 0xE0) == 0xE0;
        let layer = (b1 >> 1) & 0x03;
        if is_sync && layer != 0 {
            return AudioFormat::Mp3;
        }
    }
    AudioFormat::Unknown
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detect_webm() {
        let hdr = [0x1A, 0x45, 0xDF, 0xA3, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_format(&hdr), AudioFormat::Webm);
    }
    #[test]
    fn detect_mp4() {
        let hdr = b"\x00\x00\x00\x1Cftypisom";
        assert_eq!(detect_format(hdr), AudioFormat::Mp4);
    }
    #[test]
    fn detect_ogg() {
        assert_eq!(detect_format(b"OggS\x00"), AudioFormat::Ogg);
    }
    #[test]
    fn detect_unknown() {
        assert_eq!(
            detect_format(&[0x00, 0x00, 0x00, 0x00]),
            AudioFormat::Unknown
        );
    }
    #[test]
    fn adts_not_mistaken_for_mp3() {
        let adts = [0xFF, 0xF1, 0x50, 0x80];
        assert_eq!(detect_format(&adts), AudioFormat::Unknown);
    }
    #[test]
    fn mp3_sync_word() {
        let mp3 = [0xFF, 0xFB, 0x90, 0x00];
        assert_eq!(detect_format(&mp3), AudioFormat::Mp3);
    }
}
}
pub mod webm_opus {
use symphonia::core::{
    codecs::CODEC_TYPE_OPUS,
    errors::Error,
    formats::{FormatOptions, FormatReader},
    io::{MediaSource, MediaSourceStream},
    meta::MetadataOptions,
    probe::Hint,
};
pub struct WebmOpusDemuxer {
    format: Box<dyn FormatReader>,
    track_id: u32,
}
impl WebmOpusDemuxer {
    pub fn open(source: Box<dyn MediaSource>) -> Result<Option<Self>, Error> {
        let mss = MediaSourceStream::new(source, Default::default());
        let mut hint = Hint::new();
        hint.with_extension("webm");
        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;
        let format = probed.format;
        let track_id = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec == CODEC_TYPE_OPUS)
            .map(|t| t.id);
        Ok(track_id.map(|track_id| Self { format, track_id }))
    }
    pub fn next_packet(&mut self) -> Result<Option<Vec<u8>>, Error> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                Err(e) => return Err(e),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            return Ok(Some(packet.data.into_vec()));
        }
    }
}
}
pub use format::detect_format;
use symphonia::core::{
    codecs::{CODEC_TYPE_NULL, Decoder, DecoderOptions},
    errors::Error,
    formats::{FormatOptions, FormatReader},
    io::{MediaSource, MediaSourceStream},
    meta::MetadataOptions,
    probe::Hint,
};
pub use webm_opus::WebmOpusDemuxer;
use crate::audio::constants::{MIXER_CHANNELS, TARGET_SAMPLE_RATE};
pub use crate::common::types::AudioFormat;
pub enum DemuxResult {
    Transcode {
        format: Box<dyn FormatReader>,
        track_id: u32,
        decoder: Box<dyn Decoder>,
        sample_rate: u32,
        channels: usize,
    },
}
pub fn open_format(
    source: Box<dyn MediaSource>,
    kind: Option<AudioFormat>,
) -> Result<DemuxResult, Error> {
    let mss = MediaSourceStream::new(source, Default::default());
    let mut hint = Hint::new();
    if let Some(k) = &kind {
        hint.with_extension(k.as_ext());
    }
    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no audio track found",
            ))
        })?;
    let track_id = track.id;
    let codec = track.codec_params.codec;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(TARGET_SAMPLE_RATE);
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(MIXER_CHANNELS);
    let decoder: Box<dyn Decoder> = if codec == symphonia::core::codecs::CODEC_TYPE_OPUS {
        Box::new(
            crate::audio::codec::opus_decoder::OpusCodecDecoder::try_new(
                &track.codec_params,
                &DecoderOptions::default(),
            )?,
        )
    } else {
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?
    };
    Ok(DemuxResult::Transcode {
        format,
        track_id,
        decoder,
        sample_rate,
        channels,
    })
}