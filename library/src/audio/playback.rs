pub mod stuck_detector {
    use crate::common::utils::now_nanos;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    pub struct StuckDetector {
        last_frame_received_at_nanos: AtomicU64,
        threshold_ms: AtomicU64,
        stuck_event_sent: AtomicBool,
    }
    impl StuckDetector {
        pub fn new(threshold_ms: u64) -> Self {
            Self {
                last_frame_received_at_nanos: AtomicU64::new(now_nanos()),
                threshold_ms: AtomicU64::new(threshold_ms),
                stuck_event_sent: AtomicBool::new(false),
            }
        }
        pub fn record_frame_received(&self) {
            let now_nanos = now_nanos();
            self.last_frame_received_at_nanos
                .store(now_nanos, Ordering::Release);
        }
        pub fn reset_stuck_flag(&self) {
            self.stuck_event_sent.store(false, Ordering::Release);
        }
        pub fn check_stuck(&self) -> bool {
            if self.stuck_event_sent.load(Ordering::Acquire) {
                return false;
            }
            let now_nanos = now_nanos();
            let last_received = self.last_frame_received_at_nanos.load(Ordering::Acquire);
            let elapsed_nanos = now_nanos.saturating_sub(last_received);
            let threshold_nanos = self.threshold_ms.load(Ordering::Acquire) * 1_000_000;
            if elapsed_nanos >= threshold_nanos {
                self.stuck_event_sent.store(true, Ordering::Release);
                true
            } else {
                false
            }
        }
        pub fn threshold_ms(&self) -> u64 {
            self.threshold_ms.load(Ordering::Acquire)
        }
        pub fn set_threshold(&self, threshold_ms: u64) {
            self.threshold_ms.store(threshold_ms, Ordering::Release);
        }
    }
    impl Default for StuckDetector {
        fn default() -> Self {
            Self::new(10_000)
        }
    }
}
pub mod handle {
    use crate::audio::{constants::OPUS_SAMPLE_RATE, processor::DecoderCommand};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering},
    };
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(u8)]
    pub enum PlaybackState {
        Playing = 0,
        Paused = 1,
        Stopped = 2,
        Stopping = 3,
        Starting = 4,
    }
    impl From<u8> for PlaybackState {
        fn from(v: u8) -> Self {
            match v {
                0 => Self::Playing,
                1 => Self::Paused,
                3 => Self::Stopping,
                4 => Self::Starting,
                _ => Self::Stopped,
            }
        }
    }
    #[derive(Clone)]
    pub struct TrackHandle {
        state: Arc<AtomicU8>,
        volume: Arc<AtomicU32>,
        position: Arc<AtomicU64>,
        command_tx: flume::Sender<DecoderCommand>,
        tape_stop_enabled: Arc<AtomicBool>,
        is_buffering: Arc<AtomicBool>,
    }
    impl TrackHandle {
        pub fn new(
            command_tx: flume::Sender<DecoderCommand>,
            tape_stop_enabled: Arc<AtomicBool>,
        ) -> (
            Self,
            Arc<AtomicU8>,
            Arc<AtomicU32>,
            Arc<AtomicU64>,
            Arc<AtomicBool>,
        ) {
            let state = Arc::new(AtomicU8::new(PlaybackState::Playing as u8));
            let volume = Arc::new(AtomicU32::new(1.0f32.to_bits()));
            let position = Arc::new(AtomicU64::new(0));
            let is_buffering = Arc::new(AtomicBool::new(false));
            (
                Self {
                    state: state.clone(),
                    volume: volume.clone(),
                    position: position.clone(),
                    command_tx,
                    tape_stop_enabled,
                    is_buffering: is_buffering.clone(),
                },
                state,
                volume,
                position,
                is_buffering,
            )
        }
        pub fn pause(&self) {
            let next_state = if self.tape_stop_enabled.load(Ordering::Acquire) {
                PlaybackState::Stopping
            } else {
                PlaybackState::Paused
            };
            self.state.store(next_state as u8, Ordering::Release);
        }
        pub fn play(&self) {
            let next_state = if self.tape_stop_enabled.load(Ordering::Acquire) {
                PlaybackState::Starting
            } else {
                PlaybackState::Playing
            };
            self.state.store(next_state as u8, Ordering::Release);
        }
        pub fn stop(&self) {
            self.state
                .store(PlaybackState::Stopped as u8, Ordering::SeqCst);
        }
        pub fn set_volume(&self, vol: f32) {
            self.volume.store(vol.to_bits(), Ordering::Release);
        }
        pub fn get_state(&self) -> PlaybackState {
            let s = self.state.load(Ordering::Acquire);
            let mut state = PlaybackState::from(s);
            if state != PlaybackState::Stopped && self.command_tx.is_disconnected() {
                state = PlaybackState::Stopped;
                self.state.store(state as u8, Ordering::Release);
            }
            state
        }
        pub fn get_position(&self) -> u64 {
            let samples = self.position.load(Ordering::Acquire);
            (samples * 1000) / OPUS_SAMPLE_RATE
        }
        pub fn is_buffering(&self) -> bool {
            self.is_buffering.load(Ordering::Acquire)
        }
        pub fn seek(&self, position_ms: u64) {
            let samples = (position_ms * OPUS_SAMPLE_RATE) / 1000;
            self.position.store(samples, Ordering::Release);
            let _ = self.command_tx.send(DecoderCommand::Seek(position_ms));
        }
        pub fn is_same(&self, other: &Self) -> bool {
            self.command_tx.same_channel(&other.command_tx)
        }
    }
}
pub use handle::{PlaybackState, TrackHandle};
pub use stuck_detector::StuckDetector;
