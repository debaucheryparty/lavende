# Players & Queues (Rust)

A `LavendePlayer` represents an active audio session for a specific Discord guild. It encapsulates the voice connection, track queue, and playback state.

---

## Player Instantiation

Retrieve an existing player or create a new one for a guild.

```rust
use lavende::LavendePlayer;

let player = manager.get_or_create_player("guild_id");

// Or create explicitly
let player = manager.create_player("guild_id");

// Get existing player
if let Some(player) = manager.get_player("guild_id") {
    // Player exists
}
```

---

## Audio Resolution

Use the player's `search` method or the standalone `load` function to resolve tracks from URLs or search queries.

| `loadType` | Description |
| :--- | :--- |
| `"empty"` | The query yielded no results. |
| `"playlist"` | A collection of tracks was returned. |
| `"track"` | A single track was returned. |
| `"search"` | Multiple search results were returned. |

```rust
use lavende::load;

// Using player.search()
let result = player.search("lofi beats").await?;

// Or using standalone load()
let result = load("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()).await?;

match result.load_type.as_str() {
    "empty" => println!("No tracks found"),
    "playlist" => {
        let tracks = result.tracks;
        player.queue.write().await.add_multiple(tracks);
    }
    "track" | "search" => {
        if let Some(track) = result.tracks.first() {
            player.queue.write().await.add(track.clone());
        }
    }
    "error" => {
        eprintln!("Load error: {:?}", result.exception);
    }
    _ => {}
}
```

---

## Execution & Controls

Connect to a voice channel and begin playback.

```rust
// Connect to voice channel
player.connect(
    Some("voice_channel_id".to_string()),
    true,  // self_deaf
    false  // self_mute
).await;

// Start playback
if !player.playing {
    player.play().await?;
}
```

### Mutating the Stream

Control playback state with async methods.

| Method | Description |
| :--- | :--- |
| `await player.pause(bool)` | Pauses (`true`) or unpauses (`false`) the stream. |
| `await player.resume()` | Resumes a paused stream. |
| `await player.skip()` | Skips to the next track. |
| `await player.stop()` | Stops playback and clears current track. |
| `await player.seek(i64)` | Jumps to a millisecond timestamp. |
| `await player.set_volume(u32)` | Sets volume (0 to 100). |
| `await player.disconnect()` | Disconnects from voice channel. |
| `await player.destroy(Option<String>)` | Destroys the player and frees resources. |

```rust
// Pause playback
player.pause(true).await;

// Resume playback
player.resume().await;

// Skip to next track
player.skip().await;

// Seek to 1 minute
player.seek(60000).await;

// Set volume to 80%
player.set_volume(80).await;

// Disconnect and cleanup
player.destroy(Some("User requested".to_string())).await;
```

---

## Queue Management

The queue is protected by a `RwLock` for thread-safe access.

```rust
{
    let mut queue = player.queue.write().await;
    
    // Add single track
    queue.add(track);
    
    // Add multiple tracks
    queue.add_multiple(vec![track1, track2, track3]);
    
    // Remove track by index
    queue.remove(0);
    
    // Clear all tracks
    queue.clear();
    
    // Shuffle queue
    queue.shuffle();
    
    // Get next track
    let next = queue.next();
    
    // Queue info
    let size = queue.size();
    let is_empty = queue.is_empty();
    let total_duration = queue.total_duration();
}
```

---

## Event Handling

Events are broadcast through the manager's event channel. Subscribe to receive events for all players.

```rust
let mut events = manager.subscribe_events();

tokio::spawn(async move {
    while let Ok(event) = events.recv().await {
        match event {
            LavendeEvent::TrackStart { guild_id, track } => {
                println!("Track started: {}", track.info.title);
            }
            LavendeEvent::TrackEnd { guild_id, track, reason } => {
                println!("Track ended: {:?}", reason);
            }
            LavendeEvent::QueueEnd { guild_id } => {
                println!("Queue is empty");
            }
            LavendeEvent::Error { guild_id, message } => {
                eprintln!("Error: {}", message);
            }
            _ => {}
        }
    }
});
```

> [!TIP]
> Call `player.destroy()` when the queue ends to properly clean up resources.

---

## Repeat Mode

Control track repetition behavior.

```rust
use lavende::RepeatMode;

// No repeat
player.set_repeat_mode(RepeatMode::Off).await;

// Repeat current track
player.set_repeat_mode(RepeatMode::Track).await;

// Repeat entire queue
player.set_repeat_mode(RepeatMode::Queue).await;
```

---

## Custom Data Storage

Store arbitrary data on the player using the built-in data map.

```rust
use serde_json::json;

// Store data
player.set_data("key", json!({"value": 123}));

// Retrieve data
if let Some(data) = player.get_data("key") {
    println!("{:?}", data);
}

// Delete data
player.delete_data("key");

// Clear all data
player.clear_data();

// Get all data
let all_data = player.get_all_data();
```
