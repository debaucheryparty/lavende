# Lavende

**In-process Discord voice connection and audio playback engine**

[![npm version](https://img.shields.io/npm/v/lavende.svg)](https://www.npmjs.com/package/lavende)
[![PyPI version](https://img.shields.io/pypi/v/lavende.svg)](https://pypi.org/project/lavende/)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## Features

- **Native Performance** - Written in Rust for maximum speed and efficiency
- **No Separate Server** - Runs directly in your Node.js or Python process
- **Audio Sources** - YouTube, Spotify, SoundCloud, Apple Music, Deezer, and more
- **Audio Effects** - Equalizer, timescale, rotation, vibrato, tremolo, reverb, and more
- **Low Latency** - Direct function calls instead of REST/WebSocket overhead
- **DAVE E2EE** - Discord's end-to-end encryption support
- **Zero Dependencies** - Prebuilt binaries for all platforms
- **Multi-Language** - Available for Node.js and Python

## Installation

### Node.js

```bash
npm install lavende
```

### Python

```bash
pip install lavende
```

Lavende automatically downloads the correct prebuilt binary for your platform (Windows, macOS, Linux).

## Quick Start

### Node.js

```javascript
const { Client, GatewayIntentBits } = require('discord.js');
const { LavendeManager } = require('lavende');

const client = new Client({
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildVoiceStates,
    GatewayIntentBits.GuildMessages,
    GatewayIntentBits.MessageContent
  ]
});

const manager = new LavendeManager({
  sendToShard: (guildId, payload) => {
    client.guilds.cache.get(guildId)?.shard?.send(payload);
  },
  client: {
    id: 'YOUR_BOT_CLIENT_ID'
  }
});

client.once('ready', () => {
  console.log('Bot is ready!');
  manager.init({ id: client.user.id, username: client.user.username });
});

client.on('raw', (packet) => {
  manager.sendRawData(packet);
});

client.on('messageCreate', async (message) => {
  if (message.content === '!play') {
    const player = manager.createPlayer({
      guildId: message.guildId,
      voiceChannelId: message.member.voice.channelId,
      textChannelId: message.channelId
    });

    const result = await player.search('your song query', message.author);
    player.queue.add(result.tracks[0]);
    
    await player.connect();
    await player.play();
  }
});

client.login('YOUR_BOT_TOKEN');
```

### Python

```python
import asyncio
import discord
from lavende import LavendeManager, load

client = discord.Client(intents=discord.Intents.all())

def send_to_shard(guild_id: str, payload: dict):
    guild = client.get_guild(int(guild_id))
    if guild:
        asyncio.create_task(guild.shard.ws.send_as_json(payload))

manager = LavendeManager(
    send_to_shard=send_to_shard,
    client={"id": "YOUR_BOT_CLIENT_ID"}
)

@client.event
async def on_ready():
    print(f'Bot is ready as {client.user}')
    manager.init({"id": str(client.user.id), "username": client.user.name})

@client.event
async def on_raw_dispatch(packet):
    manager.send_raw_data(packet)

@client.event
async def on_message(message):
    if message.content == '!play':
        player = manager.create_player({
            "guild_id": str(message.guild.id),
            "voice_channel_id": str(message.author.voice.channel.id),
            "text_channel_id": str(message.channel.id)
        })
        
        result = await player.search('your song query', message.author)
        player.queue.add(result["tracks"][0])
        
        await player.connect()
        await player.play()

client.run('YOUR_BOT_TOKEN')
```

## Documentation

### Manager

**Node.js:**
```javascript
const manager = new LavendeManager({
  sendToShard: (guildId, payload) => void,
  client: { id: string, username?: string }
});

manager.init(clientData);
manager.createPlayer(options);
manager.destroyPlayer(guildId);
```

**Python:**
```python
manager = LavendeManager(
    send_to_shard=lambda guild_id, payload: None,
    client={"id": "...", "username": "..."}
)

manager.init(client_data)
manager.create_player(options)
manager.destroy_player(guild_id)
```

### Player

**Node.js:**
```javascript
const player = manager.createPlayer({
  guildId: string,
  voiceChannelId: string,
  textChannelId?: string,
  volume?: number,
  selfDeaf?: boolean
});

await player.play(options?);
await player.pause(pauseState?);
await player.resume();
await player.stop();
await player.skip();
await player.seek(positionMs);
await player.setVolume(volume);

player.queue.add(track);
player.queue.remove(index);
player.queue.clear();
player.queue.shuffle();

await player.filterManager.setVolume(volume);
await player.filterManager.setSpeed(speed);
await player.filterManager.setPitch(pitch);
await player.filterManager.setAudioOutput('mono' | 'stereo' | 'left' | 'right');
await player.filterManager.toggleRotation();
await player.filterManager.resetFilters();

const result = await player.search(query, requester);
```

**Python:**
```python
player = manager.create_player({
    "guild_id": "...",
    "voice_channel_id": "...",
    "text_channel_id": "...",
    "volume": 100,
    "self_deaf": True
})

await player.play(options)
await player.pause(pause_state)
await player.resume()
await player.stop()
await player.skip()
await player.seek(position_ms)
await player.set_volume(volume)

player.queue.add(track)
player.queue.remove(index)
player.queue.clear()
player.queue.shuffle()

await player.filter_manager.set_volume(volume)
await player.filter_manager.set_speed(speed)
await player.filter_manager.set_pitch(pitch)
await player.filter_manager.set_audio_output('mono')
await player.filter_manager.toggle_rotation()
await player.filter_manager.reset_filters()

result = await player.search(query, requester)
```

### Events

**Node.js:**
```javascript
player.on('trackStart', (player, track) => {});
player.on('trackEnd', (player, track, reason) => {});
player.on('queueEnd', (player) => {});
player.on('error', (player, error) => {});
player.on('position', (player, position) => {});
```

**Python:**
```python
player.on('track_start', lambda p, t: print(f'Track started: {t}'))
player.on('track_end', lambda p, t, r: print(f'Track ended: {r}'))
player.on('queue_end', lambda p: print('Queue ended'))
player.on('error', lambda p, e: print(f'Error: {e}'))
player.on('position', lambda p, pos: print(f'Position: {pos}'))
```

## Supported Sources

- YouTube / YouTube Music
- Spotify
- SoundCloud
- Apple Music
- Deezer
- Tidal
- JioSaavn
- Bandcamp
- Mixcloud
- Twitch
- And many more!

## Audio Filters

- Volume Control
- Equalizer (15 bands)
- Timescale (speed, pitch, rate)
- Rotation (3D audio)
- Tremolo
- Vibrato
- Distortion
- Channel Mix
- Karaoke
- Low Pass / High Pass
- Reverb
- Echo
- Chorus
- Compressor
- And more!

## Advanced Configuration

**IMPORTANT:** You must create a `source.json` file in your project root directory to run your bot. This file configures which audio sources are enabled and their settings.

Create a `source.json` file in your project root with the following configuration:

```json
{
  "sources": {
    "youtube": {
      "enabled": true,
      "clients": {
        "search": [
          "TVHTML5_SIMPLY",
          "MUSIC_ANDROID",
          "ANDROID",
          "WEB"
        ],
        "playback": [
          "ANDROID_VR",
          "TV_CAST",
          "WEB_EMBEDDED",
          "TV",
          "WEB",
          "IOS",
          "MWEB"
        ],
        "resolve": [
          "TVHTML5_SIMPLY",
          "TVHTML5_UNPLUGGED",
          "WEB",
          "MWEB",
          "IOS",
          "ANDROID"
        ]
      },
      "cipher": {
        "url": "https://cipher.kikkia.dev/api",
        "token": null
      }
    },
    "spotify": {
      "enabled": false,
      "client_id": "YOUR_SPOTIFY_CLIENT_ID",
      "client_secret": "YOUR_SPOTIFY_CLIENT_SECRET"
    },
    "soundcloud": {
      "enabled": true,
      "client_id": null
    },
    "applemusic": {
      "enabled": true,
      "country_code": "us"
    },
    "deezer": {
      "enabled": true
    },
    "jiosaavn": {
      "enabled": true
    },
    "http": {
      "enabled": true
    },
    "local": {
      "enabled": false
    }
  }
}
```

**Note:** 
- The `source.json` file is **required** for the bot to work
- Enable/disable sources by setting `"enabled": true` or `"enabled": false`
- For Spotify, you need to provide your own API credentials from [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
- For full configuration options, see the example file in the package



## Building from Source

```bash
npm install
npm run build
npm run build:debug
```

## License

Licensed under the Apache License, Version 2.0 - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please open an issue or PR.

## Acknowledgements

This project is a personal hobby and learning endeavor. It is not intended to compete with, criticize, or harm any existing projects. We have great respect for the work done by the teams behind [Lavalink](https://github.com/lavalink-devs/Lavalink), [NodeLink](https://github.com/PerformanC/NodeLink), and [lavalink-client](https://github.com/tomato6966/lavalink-client), which have been instrumental in the Discord music bot ecosystem. If you find this project useful, feel free to use it. If not, that's perfectly fine too.

Built using:
- [Lavalink](https://github.com/lavalink-devs/Lavalink) - For inspiration and protocol design
- [NodeLink](https://github.com/PerformanC/NodeLink) - For additional inspiration
- [lavalink-client](https://github.com/tomato6966/lavalink-client) - For client implementation inspiration
- [napi-rs](https://napi.rs/) - Rust bindings for Node.js
- [symphonia](https://github.com/pdm-project/symphonia) - Pure Rust audio decoding
- [audiopus](https://github.com/discord/opus) - Opus encoding
- [tokio](https://tokio.rs/) - Async runtime

---

Made by Debauchery Party
