# The Rust Manager

The `LavendeManager` is the core orchestrator for all audio sessions in Rust. It maintains active voice connections and dispatches events through Tokio's broadcast channels.

---

## Bootstrapping the Manager

The manager requires a callback function that routes Discord Gateway payloads to the correct shard.

> [!IMPORTANT]
> Initialize the manager after your Discord bot is ready and you have access to the bot's client ID.

### Initialization Parameters

| Parameter          | Type                                                       | Required | Description                                           |
| :----------------- | :--------------------------------------------------------- | :------- | :---------------------------------------------------- |
| `client_id`        | `String`                                                   | Yes      | Your bot's application/client ID.                     |
| `send_to_shard_fn` | `F: Fn(String, serde_json::Value) + Send + Sync + 'static` | Yes      | A function that routes payloads to Discord's Gateway. |

### Example Setup

```rust
use lavende::{LavendeManager, LavendeEvent};
use serde_json::json;

#[tokio::main]
async fn main() {
    let client_id = "YOUR_BOT_CLIENT_ID".to_string();

    // This closure will be called when Lavende needs to send data to Discord
    let send_to_shard = |guild_id: String, payload: serde_json::Value| {
        // In a real bot, you would send this to your Discord gateway connection
        println!("Sending to guild {}: {:?}", guild_id, payload);
    };

    let manager = LavendeManager::new(client_id, send_to_shard);

    println!("Lavende Manager initialized");
}
```

---

## Intercepting Gateway Events

Lavende requires `VOICE_STATE_UPDATE` and `VOICE_SERVER_UPDATE` Discord Gateway events to establish voice connections.

You must forward these events from your Discord library to the manager using `send_raw_data`.

```rust
use serde_json::json;

// Example: Forward a VOICE_STATE_UPDATE event
let packet = json!({
    "t": "VOICE_STATE_UPDATE",
    "d": {
        "user_id": user_id,
        "guild_id": guild_id,
        "session_id": session_id,
        "channel_id": channel_id
    }
});

manager.send_raw_data(&packet).await;

// Example: Forward a VOICE_SERVER_UPDATE event
let packet = json!({
    "t": "VOICE_SERVER_UPDATE",
    "d": {
        "guild_id": guild_id,
        "token": token,
        "endpoint": endpoint
    }
});

manager.send_raw_data(&packet).await;
```

> [!NOTE]
> The `send_raw_data` method is async and returns immediately. Lavende filters non-voice events internally, so it's safe to forward all raw Discord packets.

---

## Event Subscriptions

Lavende uses Tokio's broadcast channels for event distribution. Subscribe to receive real-time events from all players.

```rust
use lavende::LavendeEvent;

let mut events = manager.subscribe_events();

tokio::spawn(async move {
    while let Ok(event) = events.recv().await {
        match event {
            LavendeEvent::TrackStart { guild_id, track } => {
                println!("[{}] Track started: {}", guild_id, track.info.title);
            }
            LavendeEvent::TrackEnd { guild_id, track, reason } => {
                println!("[{}] Track ended: {:?}", guild_id, reason);
            }
            LavendeEvent::QueueEnd { guild_id } => {
                println!("[{}] Queue ended", guild_id);
            }
            LavendeEvent::Error { guild_id, message } => {
                eprintln!("[{}] Error: {}", guild_id, message);
            }
            LavendeEvent::Position { guild_id, position } => {
                println!("[{}] Position: {}ms", guild_id, position);
            }
            _ => {}
        }
    }
});
```

> [!TIP]
> Each call to `subscribe_events()` creates a new broadcast receiver. You can have multiple subscribers across different tasks.
