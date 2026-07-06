# Rust Library

The Lavende Rust library provides direct access to the core audio engine with full async/await support using Tokio.

## Getting Started

1. **Add Lavende to your Cargo.toml:**

   ```toml
   [dependencies]
   lavende = "0.1"
   tokio = { version = "1", features = ["full"] }
   serde_json = "1.0"
   ```

2. **Explore the Components:**
   - **[The Manager](./manager.md)**: Initialize the engine and coordinate voice connections.
   - **[Players & Queue](./player.md)**: Manage audio playback and track queuing.
   - **[Filters](./filters.md)**: Apply DSP effects to the audio stream.

> [!IMPORTANT]
> You must create a `source.json` file in your project root directory for Lavende to work. See `_examples/rust/source.json` for configuration details.

The Rust library operates entirely in-process with zero external dependencies, providing the lowest possible latency and maximum performance.
