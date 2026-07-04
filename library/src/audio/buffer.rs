pub mod pool {
    use crate::audio::constants::{MAX_BUCKET_ENTRIES, MAX_POOL_BYTES, POOL_IDLE_CLEAR_SECS};
    use parking_lot::Mutex;
    use std::{
        collections::HashMap,
        sync::OnceLock,
        time::{Duration, Instant},
    };
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(30);
    struct PoolInner {
        buckets: HashMap<usize, Vec<Vec<u8>>>,
        total_bytes: usize,
        last_activity: Instant,
        last_cleanup: Instant,
    }
    impl PoolInner {
        fn new() -> Self {
            let now = Instant::now();
            Self {
                buckets: HashMap::new(),
                total_bytes: 0,
                last_activity: now,
                last_cleanup: now,
            }
        }
        fn aligned_size(size: usize) -> usize {
            let aligned = size.max(1024).next_power_of_two();
            aligned.min(1024 * 1024)
        }
        fn needs_cleanup(&self) -> bool {
            self.total_bytes > 0 && self.last_cleanup.elapsed() >= CLEANUP_INTERVAL
        }
        fn acquire(&mut self, size: usize) -> Vec<u8> {
            self.last_activity = Instant::now();
            let aligned = Self::aligned_size(size);
            if let Some(buf) = self
                .buckets
                .get_mut(&aligned)
                .and_then(|bucket| bucket.pop())
            {
                self.total_bytes -= aligned;
                return buf;
            }
            Vec::with_capacity(aligned)
        }
        fn release(&mut self, mut buf: Vec<u8>) {
            self.last_activity = Instant::now();
            let size = buf.capacity();
            if !(1024..=10 * 1024 * 1024).contains(&size) {
                return;
            }
            if self.total_bytes + size > MAX_POOL_BYTES {
                return;
            }
            let bucket = self.buckets.entry(size).or_default();
            if bucket.len() >= MAX_BUCKET_ENTRIES {
                return;
            }
            buf.clear();
            self.total_bytes += size;
            bucket.push(buf);
        }
        fn cleanup(&mut self) {
            self.last_cleanup = Instant::now();
            let is_idle = self.last_activity.elapsed() >= Duration::from_secs(POOL_IDLE_CLEAR_SECS);
            let is_over_limit = self.total_bytes > MAX_POOL_BYTES;
            if is_idle || is_over_limit {
                self.buckets.clear();
                self.total_bytes = 0;
            }
        }
    }
    pub struct BufferPool {
        inner: Mutex<PoolInner>,
    }
    impl BufferPool {
        fn new() -> Self {
            Self {
                inner: Mutex::new(PoolInner::new()),
            }
        }
        pub fn acquire(&self, size: usize) -> Vec<u8> {
            let mut g = self.inner.lock();
            if g.needs_cleanup() {
                g.cleanup();
            }
            g.acquire(size)
        }
        pub fn release(&self, buf: Vec<u8>) {
            self.inner.lock().release(buf);
        }
        pub fn stats(&self) -> PoolStats {
            let g = self.inner.lock();
            PoolStats {
                total_bytes: g.total_bytes,
                buckets: g.buckets.len(),
                entries: g.buckets.values().map(|b| b.len()).sum(),
            }
        }
    }
    #[derive(Debug, Clone)]
    pub struct PoolStats {
        pub total_bytes: usize,
        pub buckets: usize,
        pub entries: usize,
    }
    static GLOBAL_BYTE_POOL: OnceLock<BufferPool> = OnceLock::new();
    pub fn get_byte_pool() -> &'static BufferPool {
        GLOBAL_BYTE_POOL.get_or_init(BufferPool::new)
    }
}
pub mod ring {
    use crate::audio::buffer::pool::get_byte_pool;
    pub struct RingBuffer {
        buf: Vec<u8>,
        size: usize,
        write_offset: usize,
        read_offset: usize,
        length: usize,
    }
    impl RingBuffer {
        pub fn new(size: usize) -> Self {
            let mut buf = get_byte_pool().acquire(size);
            buf.resize(size, 0);
            Self {
                buf,
                size,
                write_offset: 0,
                read_offset: 0,
                length: 0,
            }
        }
        pub fn len(&self) -> usize {
            self.length
        }
        pub fn is_empty(&self) -> bool {
            self.length == 0
        }
        pub fn remaining(&self) -> usize {
            self.size - self.length
        }
        pub fn write(&mut self, chunk: &[u8]) {
            let chunk = if chunk.len() > self.size {
                &chunk[chunk.len() - self.size..]
            } else {
                chunk
            };
            let to_write = chunk.len();
            let available_at_end = self.size - self.write_offset;
            if to_write <= available_at_end {
                self.buf[self.write_offset..self.write_offset + to_write].copy_from_slice(chunk);
            } else {
                self.buf[self.write_offset..].copy_from_slice(&chunk[..available_at_end]);
                self.buf[..to_write - available_at_end].copy_from_slice(&chunk[available_at_end..]);
            }
            let new_len = self.length + to_write;
            if new_len > self.size {
                let overwritten = new_len - self.size;
                self.read_offset = (self.read_offset + overwritten) % self.size;
                self.length = self.size;
            } else {
                self.length = new_len;
            }
            self.write_offset = (self.write_offset + to_write) % self.size;
        }
        pub fn read(&mut self, n: usize) -> Option<Vec<u8>> {
            let to_read = self.peek(n)?;
            self.read_offset = (self.read_offset + to_read.len()) % self.size;
            self.length -= to_read.len();
            Some(to_read)
        }
        pub fn peek(&self, n: usize) -> Option<Vec<u8>> {
            let to_read = n.min(self.length);
            if to_read == 0 {
                return None;
            }
            let mut out = get_byte_pool().acquire(to_read);
            out.resize(to_read, 0);
            self.copy_to(&mut out);
            Some(out)
        }
        pub fn peek_slice<F, R>(&self, n: usize, f: F) -> Option<R>
        where
            F: FnOnce(&[u8], &[u8]) -> R,
        {
            let to_read = n.min(self.length);
            if to_read == 0 {
                return None;
            }
            let available_at_end = self.size - self.read_offset;
            let result = if to_read <= available_at_end {
                f(&self.buf[self.read_offset..self.read_offset + to_read], &[])
            } else {
                f(
                    &self.buf[self.read_offset..],
                    &self.buf[..to_read - available_at_end],
                )
            };
            Some(result)
        }
        fn copy_to(&self, out: &mut [u8]) {
            let to_copy = out.len();
            let available_at_end = self.size - self.read_offset;
            if to_copy <= available_at_end {
                out.copy_from_slice(&self.buf[self.read_offset..self.read_offset + to_copy]);
            } else {
                out[..available_at_end].copy_from_slice(&self.buf[self.read_offset..]);
                out[available_at_end..].copy_from_slice(&self.buf[..to_copy - available_at_end]);
            }
        }
        pub fn skip(&mut self, n: usize) -> usize {
            let to_skip = n.min(self.length);
            self.read_offset = (self.read_offset + to_skip) % self.size;
            self.length -= to_skip;
            to_skip
        }
        pub fn clear(&mut self) {
            self.write_offset = 0;
            self.read_offset = 0;
            self.length = 0;
        }
    }
    impl Drop for RingBuffer {
        fn drop(&mut self) {
            if !self.buf.is_empty() {
                let buf = std::mem::take(&mut self.buf);
                get_byte_pool().release(buf);
            }
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn test_ring_buffer_basic() {
            let mut rb = RingBuffer::new(10);
            assert_eq!(rb.remaining(), 10);
            rb.write(b"hello");
            assert_eq!(rb.len(), 5);
            assert_eq!(rb.remaining(), 5);
            let data = rb.read(3).unwrap();
            assert_eq!(data, b"hel");
            assert_eq!(rb.len(), 2);
            let data = rb.peek(2).unwrap();
            assert_eq!(data, b"lo");
            assert_eq!(rb.len(), 2);
            let data = rb.read(5).unwrap();
            assert_eq!(data, b"lo");
            assert_eq!(rb.len(), 0);
        }
        #[test]
        fn test_ring_buffer_wrap_around() {
            let mut rb = RingBuffer::new(10);
            rb.write(b"0123456789");
            rb.skip(5);
            rb.write(b"abcde");
            let data = rb.read(10).unwrap();
            assert_eq!(data, b"56789abcde");
        }
        #[test]
        fn test_ring_buffer_overwrite() {
            let mut rb = RingBuffer::new(5);
            rb.write(b"12345");
            rb.write(b"67");
            let data = rb.read(5).unwrap();
            assert_eq!(data, b"34567");
        }
        #[test]
        fn test_ring_buffer_large_write() {
            let mut rb = RingBuffer::new(5);
            rb.write(b"12345678");
            let data = rb.read(5).unwrap();
            assert_eq!(data, b"45678");
        }
        #[test]
        fn test_peek_slice_zero_copy() {
            let mut rb = RingBuffer::new(10);
            rb.write(b"hello");
            let result = rb.peek_slice(5, |a, b| {
                let mut v = a.to_vec();
                v.extend_from_slice(b);
                v
            });
            assert_eq!(result.unwrap(), b"hello");
        }
    }
}
pub use pool::{BufferPool, get_byte_pool};
pub use ring::RingBuffer;
pub type PooledBuffer = Vec<i16>;
pub fn cast_to_bytes(v: PooledBuffer) -> Vec<u8> {
    let mut v = std::mem::ManuallyDrop::new(v);
    unsafe { Vec::from_raw_parts(v.as_mut_ptr() as *mut u8, v.len() * 2, v.capacity() * 2) }
}
pub fn cast_from_bytes(v: Vec<u8>) -> PooledBuffer {
    debug_assert_eq!(v.len() % 2, 0, "byte buffer length must be even");
    debug_assert_eq!(v.capacity() % 2, 0, "byte buffer capacity must be even");
    let mut v = std::mem::ManuallyDrop::new(v);
    unsafe { Vec::from_raw_parts(v.as_mut_ptr() as *mut i16, v.len() / 2, v.capacity() / 2) }
}
#[inline]
pub fn as_byte_slice(v: &[i16]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2) }
}
#[inline]
pub fn as_i16_slice(v: &[u8]) -> &[i16] {
    debug_assert_eq!(
        v.as_ptr() as usize % std::mem::align_of::<i16>(),
        0,
        "byte slice must be 2-byte aligned for i16 reinterpretation"
    );
    debug_assert_eq!(v.len() % 2, 0, "byte slice length must be even");
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const i16, v.len() / 2) }
}
#[inline]
pub fn release_buffer(v: PooledBuffer) {
    get_byte_pool().release(cast_to_bytes(v));
}
#[inline]
pub fn acquire_buffer(capacity: usize) -> PooledBuffer {
    cast_from_bytes(get_byte_pool().acquire(capacity * 2))
}
