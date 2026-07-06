# Python Wrapper

The Lavende Python wrapper offers a high-level API over the low-level FFI (Foreign Function Interface) library. It seamlessly connects with `discord.py`, `pycord`, or any asyncio-based Discord library.

## Getting Started

1. **Install the package:**
   ```bash
   pip install -e .
   ```

2. **Explore the Components:**
   - **[The Manager](./manager.md)**: Handling the core instance and WebSockets.
   - **[Players & Queue](./player.md)**: Resolving and streaming audio.
   - **[Filters](./filters.md)**: Using the native DSP engine.

> [!IMPORTANT]
> You must create a `source.json` file in your project root directory for Lavende to work. See `_examples/python/source.json` for configuration details.

The Python wrapper employs asyncio internally to ensure that operations like track resolution do not block the Python Global Interpreter Lock (GIL). However, the actual streaming and processing thread runs completely outside Python, within Rust.
