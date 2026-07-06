# Golang Wrapper

The Lavende Go package (`lavende`) bridges the gap between Go and the underlying Rust library using CGO. It is designed to work seamlessly with Go Discord libraries such as `discordgo`.

## Getting Started

1. **Install the package:**
   Ensure you have a working C compiler in your environment, as it compiles against `lavende_go.a`.
   ```bash
   go get lavende
   ```

2. **Explore the Components:**
   - **[The Manager](./manager.md)**: Bootstrapping the engine and routing gateway updates.
   - **[Players & Queue](./player.md)**: Handling guild audio and searches.
   - **[Filters](./filters.md)**: Applying real-time audio manipulation.

> [!IMPORTANT]
> You must create a `source.json` file in your project root directory for Lavende to work. See `_examples/golang/source.json` for configuration details.

Because Lavende leverages Go's concurrency model alongside Rust's multi-threading, this implementation is exceptionally efficient, avoiding any blocking behaviors on your main application thread.