pub mod udp {
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use xsalsa20poly1305::{XSalsa20Poly1305, aead::{AeadInPlace, KeyInit}};
pub const RTP_VERSION_BYTE: u8 = 0x80;
pub const RTP_OPUS_PAYLOAD_TYPE: u8 = 0x78;
pub const RTP_TIMESTAMP_STEP: u32 = 960;
pub const DISCOVERY_PACKET_SIZE: usize = 74;
pub async fn discover_ip(
    socket: &UdpSocket,
    addr: SocketAddr,
    ssrc: u32,
) -> Result<(String, u16), String> {
    let mut packet = [0u8; DISCOVERY_PACKET_SIZE];
    packet[0..2].copy_from_slice(&1u16.to_be_bytes());
    packet[2..4].copy_from_slice(&70u16.to_be_bytes());
    packet[4..8].copy_from_slice(&ssrc.to_be_bytes());
    for attempt in 1..=10 {
        if attempt > 1 {
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
        if let Err(e) = socket.send_to(&packet, addr).await {
            if attempt == 10 {
                return Err(format!("Discovery send error: {e}"));
            }
            continue;
        }
        let mut client_buf = [0u8; DISCOVERY_PACKET_SIZE];
        match tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut client_buf)).await {
            Ok(Ok((n, peer))) if n >= DISCOVERY_PACKET_SIZE => {
                if peer != addr {
                    continue;
                }
                let ip = std::str::from_utf8(&client_buf[8..72])
                    .map_err(|e| format!("IP parse error: {e}"))?
                    .trim_end_matches('\0')
                    .to_owned();
                let port = u16::from_be_bytes([client_buf[72], client_buf[73]]);
                return Ok((ip, port));
            }
            _ => {
                if attempt == 10 {
                    return Err("Discovery timeout".to_string());
                }
            }
        }
    }
    Err("Discovery exhausted".to_string())
}
#[derive(Debug, Clone, Copy)]
pub struct RtpState {
    pub sequence: u16,
    pub timestamp: u32,
}
impl RtpState {
    pub fn new() -> Self {
        Self {
            sequence: rand::random(),
            timestamp: rand::random(),
        }
    }
    pub fn next(&mut self) -> (u16, u32) {
        let seq = self.sequence;
        let ts = self.timestamp;
        self.sequence = self.sequence.wrapping_add(1);
        self.timestamp = self.timestamp.wrapping_add(RTP_TIMESTAMP_STEP);
        (seq, ts)
    }
}
pub struct UDPVoiceTransport {
    socket: Arc<UdpSocket>,
    address: SocketAddr,
    ssrc: u32,
    cipher: XSalsa20Poly1305,
    rtp: RtpState,
    buffer: Vec<u8>,
}
impl UDPVoiceTransport {
    pub fn new(
        socket: Arc<UdpSocket>,
        address: SocketAddr,
        ssrc: u32,
        secret_key: [u8; 32],
    ) -> Self {
        let cipher = XSalsa20Poly1305::new(&secret_key.into());
        Self {
            socket,
            address,
            ssrc,
            cipher,
            rtp: RtpState::new(),
            buffer: Vec::with_capacity(1500),
        }
    }
    pub async fn transmit_opus(&mut self, opus_data: &[u8]) -> Result<(), String> {
        let (seq, ts) = self.rtp.next();
        let mut header = [0u8; 12];
        header[0] = RTP_VERSION_BYTE;
        header[1] = RTP_OPUS_PAYLOAD_TYPE;
        header[2..4].copy_from_slice(&seq.to_be_bytes());
        header[4..8].copy_from_slice(&ts.to_be_bytes());
        header[8..12].copy_from_slice(&self.ssrc.to_be_bytes());
        self.buffer.clear();
        self.buffer.extend_from_slice(&header);
        self.buffer.extend_from_slice(opus_data);
        let mut nonce = [0u8; 24];
        nonce[0..12].copy_from_slice(&header);
        let tag = self.cipher
            .encrypt_in_place_detached(&nonce.into(), &header, &mut self.buffer[12..])
            .map_err(|e| format!("XSalsa20 encryption error: {e:?}"))?;
        self.buffer.extend_from_slice(&tag);
        self.socket.send_to(&self.buffer, self.address).await
            .map_err(|e| format!("UDP send error: {e}"))?;
        Ok(())
    }
    pub async fn send_keepalive(&self) -> Result<(), String> {
        let payload = [0u8; 8];
        self.socket.send_to(&payload, self.address).await
            .map_err(|e| format!("Keepalive send error: {e}"))?;
        Ok(())
    }
}
}