# System Architecture

Understanding the internal architecture of Lavende is crucial for optimizing your application and effectively utilizing the API. The system operates on a dual-layer architecture: a highly concurrent **Native Rust Core**, and a lightweight **Language Interop Layer**.

---

## The Native Rust Core

The core engine (`liblavende`) handles operations that are typically bottlenecks in managed languages (like Node.js, Python, or Go).

### 1. The Gateway Orchestrator

Unlike traditional libraries that require an external JVM application, Lavende embeds its own connection orchestrator. Once it receives raw Discord `VOICE_SERVER_UPDATE` and `VOICE_STATE_UPDATE` packets, the orchestrator:

- Resolves the Guild's designated Voice WebSocket endpoint.
- Negotiates the connection and performs the IP Discovery phase.
- Exchanges the secret key used for encrypting RTP packets using `xsalsa20_poly1305` encryption.

### 2. Audio Processing Pipeline

When a track is requested, the pipeline executes the following sequence:

1. **Resolution**: Queries external APIs (e.g., YouTube, SoundCloud) to find streamable URLs.
2. **Decoding**: Pulls bytes over HTTPS and decodes formats (Opus, AAC, MP3) into raw PCM data.
3. **DSP Engine**: Passes the raw PCM data through the Digital Signal Processing chain (applying Equalizers, time-stretching, etc.).
4. **Encoding & Transport**: Re-encodes the manipulated PCM data into Discord-compliant 48kHz Opus frames and ships them over the UDP socket.

> [!IMPORTANT]
> Because all of this occurs entirely within Rust's highly optimized runtime, the garbage collector of your host language is never stressed. The audio stream remains flawless and un-interrupted regardless of what your main application thread is doing.

---

## The FFI Interop Layer

To communicate with the Rust Core, Lavende utilizes Foreign Function Interfaces (FFI).

When you execute a command in your language (e.g., `player.pause(true)` in Python), the following occurs:

1. The wrapper marshals the request into a C-compatible format.
2. The pointer is passed across the FFI boundary to a `#[no_mangle] extern "C"` function in Rust.
3. Rust securely acquires the lock on the specific `LavendePlayer` instance in memory and mutates its state.
4. If an event occurs in Rust (e.g., a track finishes), it triggers a C-callback mapped to your language's event loop, seamlessly resuming execution in your environment.

---

## Lifecycle of a Stream

1. **Manager Initialization**: You initialize `LavendeManager`. The FFI boundary is established.
2. **Event Routing**: Your bot connects to Discord. You pipe all raw gateway payloads to the `LavendeManager`.
3. **Player Instantiation**: A user runs `/play`. You request a `Player` for the guild. Rust allocates memory for the audio session.
4. **Track Execution**: You push a track into the Queue. Rust automatically connects to the UDP socket and begins streaming.
5. **Teardown**: The queue empties, or the user runs `/stop`. The `Player` is destroyed, and the Rust core frees the allocated memory safely.
