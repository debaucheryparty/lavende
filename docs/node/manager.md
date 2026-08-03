# The Node.js Manager

The `LavendeManager` is the primary entry point for the Node.js wrapper. It sits at the top level of your application, maintaining references to all active Voice UDP connections and orchestrating the FFI boundary.

---

## Bootstrapping the Manager

Because Lavende abstracts away the complexities of WebSocket communication, you must provide it with a callback that routes payloads back to Discord's Voice Gateway.

> [!IMPORTANT]
> The manager must be instantiated globally, preferably inside your `client.once('ready')` block to ensure `client.user` is populated.

### Initialization Parameters

| Parameter     | Type                                      | Required | Description                                                               |
| :------------ | :---------------------------------------- | :------- | :------------------------------------------------------------------------ |
| `sendToShard` | `(guildId: string, payload: any) => void` | Yes      | A function that routes a raw JSON payload to the correct WebSocket shard. |
| `client`      | `{ id: string, username: string }`        | Yes      | An object containing `{ id, username }` of your bot.                      |

### Configuration

Set the source configuration file path before initializing the manager:

```typescript
import { setConfigPath } from "lavende";
setConfigPath("./config/lavende.json");
```

If not set, defaults to `source.json` in the current directory.

### Example Setup

You can use Lavende in standard JavaScript or heavily-typed TypeScript environments.

<details open>
<summary><b>TypeScript Setup</b></summary>

```typescript
import { Client, GatewayIntentBits } from "discord.js";
import { LavendeManager, setConfigPath } from "lavende";

const client = new Client({
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildMessages,
    GatewayIntentBits.MessageContent,
    GatewayIntentBits.GuildVoiceStates,
  ],
});

let manager: LavendeManager | null = null;

client.once("ready", () => {
  if (!client.user) return;

  // Optional: Set custom config path
  setConfigPath("./config/lavende.json");

  manager = new LavendeManager({
    sendToShard: (guildId: string, payload: any) => {
      client.guilds.cache.get(guildId)?.shard?.send(payload);
    },
    client: {
      id: client.user.id,
      username: client.user.username,
    },
  });

  manager.init();
  console.log("Lavende Manager Initialized Successfully");
});
```

</details>

<details>
<summary><b>JavaScript Setup</b></summary>

```javascript
const { Client, GatewayIntentBits } = require("discord.js");
const { LavendeManager, setConfigPath } = require("lavende");

const client = new Client({
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildMessages,
    GatewayIntentBits.MessageContent,
    GatewayIntentBits.GuildVoiceStates,
  ],
});

let manager = null;

client.once("ready", () => {
  // Optional: Set custom config path
  setConfigPath("./config/lavende.json");

  manager = new LavendeManager({
    sendToShard: (guildId, payload) => {
      client.guilds.cache.get(guildId)?.shard?.send(payload);
    },
    client: {
      id: client.user.id,
      username: client.user.username,
    },
  });

  manager.init();
  console.log("Lavende Manager Initialized Successfully");
});
```

</details>

---

## Intercepting Gateway Events

Lavende requires two specific Discord Gateway events to negotiate a voice connection: `VOICE_STATE_UPDATE` and `VOICE_SERVER_UPDATE`.

You must intercept raw socket payloads and pipe them directly into the manager.

```typescript
// TypeScript & JavaScript
client.on("raw", async (packet: any) => {
  if (manager) {
    // The Rust core will safely ignore non-voice events (like MESSAGE_CREATE)
    manager.sendRawData(packet);
  }
});
```

> [!NOTE]
> `sendRawData` executes virtually instantaneously. Passing the firehose of raw events into it will not bottleneck your Node.js event loop because unhandled events are discarded immediately at the C-boundary.
