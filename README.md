[![npm version](https://img.shields.io/npm/v/lavende.svg?color=cb3837)](https://www.npmjs.com/package/lavende)
[![PyPI version](https://img.shields.io/pypi/v/lavende.svg?color=3775a9)](https://pypi.org/project/lavende/)
[![Crates.io](https://img.shields.io/crates/v/lavende.svg?color=fc8d62)](https://crates.io/crates/lavende)
[![Go Reference](https://pkg.go.dev/badge/github.com/debaucheryparty/lavende.svg)](https://pkg.go.dev/github.com/debaucheryparty/lavende)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/debaucheryparty/lavende/blob/master/LICENSE)
[![Release](https://img.shields.io/github/v/tag/debaucheryparty/lavende?label=release)](https://github.com/debaucheryparty/lavende/releases/latest)

<img align="right" src="/archive/lavende.png" width=192 alt="lavende logo">

# Lavende

Lavende is a high-performance native audio processing library designed specifically for [Discord](https://discord.com) bots. It completely replaces traditional external JVM-based nodes by running a native Rust core directly inside your bot's process.

## Summary

1. [Features](#features)
2. [Getting Started](#getting-started)
3. [Documentation](#documentation)
4. [Examples](#examples)
5. [License](#license)

### Features

- [x] **Native Performance**: Engineered entirely in Rust for a minimal memory footprint and blistering execution speed.
- [x] **Zero-IPC Latency**: Executes directly inside your application without REST or WebSocket overhead.
- [x] **Embedded DSP Engine**: Apply Nightcore, Vaporwave, Bassboost, Equalizers, and 3D Rotation effortlessly on the native audio stream.
- [x] **[DAVE (E2EE)](https://discord.com/developers/docs/topics/voice-connections#endtoend-encryption-dave-protocol)**: Full support for Discord's End-to-End Encryption protocol out of the box.
- [x] **Multi-Language Support**: Fully functional and strictly typed wrappers for **Node.js**, **Python**, **Golang**, and **Rust**.
- [x] **Audio Sources**: Built-in resolution for YouTube, SoundCloud, Spotify, Apple Music, and Deezer via internal C-bindings.

## Getting Started

### Installing

Depending on your language, install Lavende via your package manager:

**Node.js**:

```sh
$ npm install lavende
```

**Python**:

```sh
$ pip install lavende
```

**Golang**:

```sh
$ go get github.com/debaucheryparty/lavende
```

**Rust**:

```sh
$ cargo add lavende
```

### Initializing Lavende

Lavende sits side-by-side with your Discord API wrapper (e.g., discord.js, discord.py, discordgo). You simply pipe raw voice socket events into the Lavende Manager so that the Rust core can establish the UDP connection.

Here is a quick Node.js initialization strategy:

```javascript
const { Client, GatewayIntentBits } = require("discord.js");
const { LavendeManager } = require("lavende");

const client = new Client({
  intents: [GatewayIntentBits.Guilds, GatewayIntentBits.GuildVoiceStates],
});
let manager = null;

client.once("ready", () => {
  manager = new LavendeManager({
    sendToShard: (guildId, payload) => {
      client.guilds.cache.get(guildId)?.shard?.send(payload);
    },
    client: { id: client.user.id, username: client.user.username },
  });
  manager.init();
});

// Pass raw socket events to Lavende to establish the Voice connection
client.on("raw", (packet) => {
  if (manager) manager.sendRawData(packet);
});

client.login("token");
```

## Quick Start

1. Clone the repository:
```bash
git clone https://github.com/debaucheryparty/lavende.git
cd lavende
````

2. Choose your preferred language from the `_examples` directory:

   * Rust
   * JavaScript / TypeScript
   * Go
   * Python

3. Create a `.env` file and add your Discord bot token:

```env
TOKEN=YOUR_BOT_TOKEN
```

4. Install dependencies and run the example for your chosen language.

### Node.js

```bash
cd _examples/node
npm install
node bot.js
```

### Python

```bash
cd _examples/python
pip install -r requirements.txt
python main.py
```

### Go

```bash
cd _examples/go
go mod tidy
go run .
```

### Rust

```bash
cd _examples/rust
cargo run
```

For detailed guides, API documentation, and architecture information, see the `docs/` directory.

## Documentation

Extensive documentation detailing the Rust architecture, the `LavendeManager`, `Player`, and `Filters` can be found in our official documentation directory:

- [x] [**Read the Lavende Documentation**](./docs/README.md)

## Examples

The best way to understand how to build a bot with Lavende is to analyze the official boilerplate examples:

- [x] [Node.js / TypeScript Bot](./_examples/node/)
- [x] [Python Bot](./_examples/python/)
- [x] [Golang Bot](./_examples/golang/)
- [x] [Rust Bot](./_examples/rust/)

## License

Distributed under the [![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/debaucheryparty/lavende/blob/master/LICENSE). See LICENSE for more information.
