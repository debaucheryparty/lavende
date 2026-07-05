# Node.js Wrapper

The Lavende Node.js wrapper provides a seamless integration with Discord libraries like `discord.js`.

## Getting Started

1. **Install the package:**
   ```bash
   npm install lavende
   ```

2. **Explore the Components:**
   - **[The Manager](./manager.md)**: Initialize the engine and route WebSocket events.
   - **[Players & Queue](./player.md)**: Manage audio playback and track queuing.
   - **[Filters](./filters.md)**: Apply DSP effects to the audio stream.

Make sure you have Node.js 18+ installed. The wrapper internally utilizes native `.node` modules compiled via N-API, meaning it runs optimally without requiring external JVM installations.
