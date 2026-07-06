# Lavende Documentation

Lavende is a high-performance, native audio processing engine tailored for Discord applications. It was engineered from the ground up to replace external JVM-based audio nodes (such as Lavalink) by embedding a highly optimized Rust core directly into your application's process.

---

## Core Philosophy

Traditional Discord audio bot architectures require maintaining an external server that handles RTP packing, voice socket connections, and audio decoding. This approach introduces significant overhead in deployment, latency, and memory utilization.

Lavende shifts this paradigm by providing a unified binary layer via Foreign Function Interfaces (FFI) and CGO.

> [!NOTE]
> **Key Takeaway**
> With Lavende, your audio processing runs exactly where your bot runs. You get the memory safety and blistering speed of Rust, combined with the development speed of your preferred high-level language.

---

## Technical Advantages

| Feature              | Description                                                                                                                                |
| :------------------- | :----------------------------------------------------------------------------------------------------------------------------------------- |
| **Zero-IPC Latency** | Audio events and commands bypass external WebSocket layers. Commands execute instantly against the native memory space.                    |
| **Micro-Footprint**  | Operating without the JVM means memory consumption drops from hundreds of megabytes to mere fractions of that.                             |
| **Embedded DSP**     | Real-time audio manipulation (Equalizers, spatial audio, time-stretching) is handled natively without shelling out to `ffmpeg`.            |
| **API Symmetry**     | The API surface is meticulously standardized. A script written in Python reads almost identically to its Rust, Go, or Node.js counterpart. |

---

## Documentation Directory

Navigate to your language of choice to explore detailed integration guides, API references, and best practices.

### 1. Conceptual Architecture

- [**Core Architecture & FFI**](./architecture.md) — Dive into the internals of the Rust bridge and UDP orchestration.

### 2. Language References

- [**Node.js Integration**](./node/README.md) — Complete guide for V8 / JavaScript environments.
- [**Python Integration**](./python/README.md) — Complete guide for asyncio-based Python applications.
- [**Golang Integration**](./golang/README.md) — Complete guide leveraging CGO and goroutines.
- [**Rust Integration**](./rust/README.md) — Complete guide for native Rust applications.

> [!TIP]
> If you are migrating from Lavalink or a similar service, we highly recommend reading the [Architecture](./architecture.md) page first to understand the shift from an external microservice model to an embedded native model.
