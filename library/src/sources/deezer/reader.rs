use crate::{
    audio::source::{AudioSource, HttpSource, create_client},
    common::types::AnyResult,
};
use cbc::cipher::{BlockDecryptMut, KeyIvInit};
use md5::{Digest, Md5};
use std::io::{Read, Seek, SeekFrom};
use symphonia::core::io::MediaSource;
use tracing::warn;

type BlowfishCbc = cbc::Decryptor<blowfish::Blowfish>;
pub const CHUNK_SIZE: usize = 2048;

pub struct DeezerCrypt {
    key: [u8; 16],
}

impl DeezerCrypt {
    pub fn new(track_id: &str, master_key: &str) -> Self {
        let hash = Md5::digest(track_id.as_bytes());
        let hash_hex = hex::encode(hash);
        let hash_bytes = hash_hex.as_bytes();
        let master_bytes = master_key.as_bytes();
        let mut key = [0u8; 16];
        for i in 0..16 {
            key[i] = hash_bytes[i] ^ hash_bytes[i + 16] ^ master_bytes[i];
        }
        Self { key }
    }

    pub fn decrypt_chunk(&self, chunk_index: u64, chunk: &[u8], dest: &mut Vec<u8>) {
        if chunk_index % 3 == 0 {
            let iv = [0, 1, 2, 3, 4, 5, 6, 7];
            let mut buffer = [0u8; CHUNK_SIZE];
            let len = std::cmp::min(chunk.len(), CHUNK_SIZE);
            buffer[..len].copy_from_slice(&chunk[..len]);
            if let Ok(cipher) = BlowfishCbc::new_from_slices(&self.key, &iv) {
                if let Ok(decrypted) =
                    cipher.decrypt_padded_mut::<cbc::cipher::block_padding::NoPadding>(&mut buffer)
                {
                    dest.extend_from_slice(decrypted);
                    return;
                } else {
                    warn!(
                        "Blowfish decryption failed for chunk {}, falling back to raw",
                        chunk_index
                    );
                }
            }
        }
        dest.extend_from_slice(chunk);
    }
}

pub struct DeezerRemoteReader {
    inner: HttpSource,
}

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36";

impl DeezerRemoteReader {
    pub async fn new(
        url: &str,
        local_addr: Option<std::net::IpAddr>,
        proxy: Option<crate::config::HttpProxyConfig>,
    ) -> AnyResult<Self> {
        let client = create_client(USER_AGENT.to_owned(), local_addr, proxy, None)?;
        let inner = HttpSource::new(client, url).await?;
        Ok(Self { inner })
    }

    pub fn content_type(&self) -> Option<String> {
        self.inner.content_type()
    }
}

impl Read for DeezerRemoteReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for DeezerRemoteReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl MediaSource for DeezerRemoteReader {
    fn is_seekable(&self) -> bool {
        self.inner.is_seekable()
    }

    fn byte_len(&self) -> Option<u64> {
        self.inner.byte_len()
    }
}

pub struct DeezerReader {
    source: DeezerRemoteReader,
    crypt: DeezerCrypt,
    pos: u64,
    raw_buf: Vec<u8>,
    ready_buf: Vec<u8>,
    skip_pending: usize,
}

impl DeezerReader {
    pub async fn new(
        url: &str,
        track_id: &str,
        master_key: &str,
        local_addr: Option<std::net::IpAddr>,
        proxy: Option<crate::config::HttpProxyConfig>,
    ) -> AnyResult<Self> {
        tracing::debug!("Initializing DeezerReader for track {}", track_id);
        let source = DeezerRemoteReader::new(url, local_addr, proxy).await?;
        let crypt = DeezerCrypt::new(track_id, master_key);
        Ok(Self {
            source,
            crypt,
            pos: 0,
            raw_buf: Vec::with_capacity(CHUNK_SIZE * 2),
            ready_buf: Vec::with_capacity(CHUNK_SIZE * 2),
            skip_pending: 0,
        })
    }
}

impl Read for DeezerReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.skip_pending > 0 && !self.ready_buf.is_empty() {
                let to_skip = std::cmp::min(self.skip_pending, self.ready_buf.len());
                self.ready_buf.drain(..to_skip);
                self.skip_pending -= to_skip;
            }
            if self.skip_pending == 0 && !self.ready_buf.is_empty() {
                let n = std::cmp::min(buf.len(), self.ready_buf.len());
                buf[..n].copy_from_slice(&self.ready_buf[..n]);
                self.ready_buf.drain(..n);
                return Ok(n);
            }
            let mut tmp = [0u8; CHUNK_SIZE];
            let n = self.source.read(&mut tmp)?;
            if n == 0 {
                if self.raw_buf.is_empty() {
                    return Ok(0);
                }
                let leftovers = self.raw_buf.clone();
                let chunk_idx = self.pos / CHUNK_SIZE as u64;
                self.crypt
                    .decrypt_chunk(chunk_idx, &leftovers, &mut self.ready_buf);
                self.pos += leftovers.len() as u64;
                self.raw_buf.clear();
                continue;
            }
            self.raw_buf.extend_from_slice(&tmp[..n]);
            while self.raw_buf.len() >= CHUNK_SIZE {
                let chunk: Vec<u8> = self.raw_buf.drain(..CHUNK_SIZE).collect();
                let chunk_idx = self.pos / CHUNK_SIZE as u64;
                self.crypt
                    .decrypt_chunk(chunk_idx, &chunk, &mut self.ready_buf);
                self.pos += CHUNK_SIZE as u64;
            }
        }
    }
}

impl Seek for DeezerReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::Current(0) => {
                let buffered = self.ready_buf.len() as u64 + self.raw_buf.len() as u64;
                return Ok(self.pos.saturating_sub(buffered));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Only SeekFrom::Start is supported",
                ));
            }
        };
        let aligned_pos = (target / CHUNK_SIZE as u64) * CHUNK_SIZE as u64;
        let skip = (target - aligned_pos) as usize;
        let new_pos = self.source.seek(SeekFrom::Start(aligned_pos))?;
        self.pos = new_pos;
        self.raw_buf.clear();
        self.ready_buf.clear();
        self.skip_pending = skip;
        Ok(target)
    }
}

impl MediaSource for DeezerReader {
    fn is_seekable(&self) -> bool {
        self.source.is_seekable()
    }

    fn byte_len(&self) -> Option<u64> {
        self.source.byte_len()
    }
}
