# The Python Manager

The `LavendeManager` is the central control plane for your Discord application. It holds the reference to the Rust Core and maintains the map of active guild sessions (Players).

---

## Bootstrapping the Manager

Lavende relies on your Discord client to handle WebSocket communication. You must initialize the manager with a callback function that instructs Lavende on how to send payloads back to Discord's gateway.

> [!IMPORTANT]
> The manager must be instantiated globally, typically inside your bot's `on_ready` event, to ensure the event loop is active and the client user data is fully populated.

### Initialization Parameters

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `send_to_shard` | `Callable[[str, dict], Coroutine]` | Yes | An async function that routes a payload to the correct WebSocket shard. |
| `client` | `Dict[str, str]` | Yes | A dictionary containing `"id"` and `"username"` of your bot. |

### Example Setup

```python
import discord
from discord.ext import commands
from lavende import LavendeManager

bot = commands.Bot(command_prefix="!", intents=discord.Intents.all())
manager = None

async def send_to_shard(guild_id: str, payload: dict):
    """Routes the raw JSON payload to the appropriate Discord Shard."""
    guild = bot.get_guild(int(guild_id))
    shard_id = guild.shard_id if guild else 0
    ws = bot.shards.get(shard_id) if hasattr(bot, 'shards') else bot.ws
    if ws:
        await ws.send_as_json(payload)

@bot.event
async def on_ready():
    global manager
    manager = LavendeManager(
        send_to_shard=send_to_shard,
        client={
            "id": str(bot.user.id), 
            "username": bot.user.name
        }
    )
    manager.init()
    print("Lavende Manager Initialized Successfully")
```

---

## Intercepting Gateway Events

For Lavende to connect to a Discord voice server, it needs the Session ID and the Voice Token. Discord provides these via `VOICE_STATE_UPDATE` and `VOICE_SERVER_UPDATE` WebSocket events.

You must intercept raw socket payloads and pipe them directly into the manager.

```python
@bot.event
async def on_socket_raw_receive(msg):
    """Pipes all raw WebSocket traffic into the Lavende Manager."""
    if not manager:
        return
        
    import json
    try:
        # Decode the byte payload if necessary
        raw = msg.decode('utf-8') if isinstance(msg, bytes) else msg
        packet = json.loads(raw)
        
        # The Rust core will safely ignore non-voice events
        await manager.send_raw_data(packet)
    except Exception as e:
        # Silently drop malformed packets
        pass
```

> [!NOTE]
> `send_raw_data` is highly optimized in Rust. Passing non-voice events (like `MESSAGE_CREATE`) has practically zero performance impact, as they are discarded immediately at the boundary.
