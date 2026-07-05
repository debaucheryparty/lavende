# Players & Queues (Python)

A `Player` encapsulates the audio session for a single Discord Guild. It manages the Voice UDP connection, the internal queue, and audio control logic.

---

## Player Instantiation

When a user executes a command (e.g., `/play`), you should query the `LavendeManager` for an existing player. If one does not exist, create it.

```python
player = manager.players.get(guild_id)

if not player:
    player = manager.create_player({
        "guild_id": guild_id,
        "voice_channel_id": voice_channel_id,
        "text_channel_id": text_channel_id,
        "volume": 100 # Volume percentage (0 to 1000)
    })
```

---

## Audio Resolution

Lavende features an internal resolver that can translate raw queries or direct URLs into playable Track objects. The resolver response includes a `loadType` defining what was returned.

| `loadType` | Description |
| :--- | :--- |
| `empty` | The query yielded no results. |
| `playlist` | A collection of tracks was returned (e.g., a YouTube playlist). |
| `track` | A single track or search result was returned. |

```python
# The resolver expects the query and an arbitrary "requester" object
res = await player.search("lofi hip hop radio", message.author)
load_type = res.get("loadType")

if load_type == "empty":
    return print("No tracks found.")

if load_type == "playlist":
    # Enqueue the entire playlist
    player.queue.add(res.get("tracks", []))
else:
    # Enqueue the single track
    player.queue.add(res.get("tracks", [])[0])
```

---

## Execution & Controls

If the player is currently idle, you must explicitly tell it to connect and begin processing the queue.

```python
if not player.playing:
    await player.connect()
    await player.play()
```

### Mutating the Stream

You can mutate the state of the active audio stream using standard asynchronous methods.

| Method | Description |
| :--- | :--- |
| `await player.pause(True/False)` | Pauses or unpauses the stream. |
| `await player.resume()` | Resumes a paused stream. |
| `await player.skip()` | Skips to the next track in the queue. |
| `await player.destroy()` | Destroys the C-pointer, clears the queue, and drops the voice connection. |
| `await player.seek(ms: int)` | Jumps to a specific millisecond timestamp in the current track. |
| `await player.set_volume(vol: int)`| Updates the volume (0 to 1000). |

---

## Event Subscriptions

Lavende operates asynchronously and emits lifecycle events. You can subscribe to these events using the `.on()` method.

> [!TIP]
> Make sure to call `player.destroy()` on `queue_end` to clean up resources effectively when playback finishes.

```python
async def on_track_start(player, track):
    print(f"Now playing: {track.info.title} requested by {track.requester}")

async def on_track_end(player, track, reason):
    # 'reason' string can be 'finished', 'stopped', 'replaced', 'loadFailed'
    pass

async def on_queue_end(player):
    # The queue is empty, clean up the session
    await player.destroy()

async def on_error(player, error_message):
    print(f"A native rust error occurred: {error_message}")

# Attach the hooks to the player instance
player.on('track_start', on_track_start)
player.on('track_end', on_track_end)
player.on('queue_end', on_queue_end)
player.on('error', on_error)
```
