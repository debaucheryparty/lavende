# Players & Queues (Node.js)

The `Player` represents an active audio session bound to a specific Discord Guild. It encapsulates the underlying Voice UDP socket, the track queue, and the playback logic.

---

## Player Instantiation

When you receive a play command, you should retrieve an existing player or create one if the guild currently has no active session.

```javascript
let player = manager.players.get(guildId);

if (!player) {
    player = manager.createPlayer({
        guildId: guildId,
        voiceChannelId: voiceChannelId,
        textChannelId: textChannelId, // Where to send "Now Playing" messages
        volume: 100 // Scale from 0 to 1000
    });
}
```

---

## Resolving Audio

Lavende utilizes an internal resolution system that bypasses standard HTTP overhead by parsing metadata natively in Rust.

| `loadType` | Description |
| :--- | :--- |
| `empty` | The query yielded no results. |
| `playlist` | A collection of tracks was returned (e.g., a YouTube playlist). |
| `track` | A single track or search result was returned. |

```javascript
// The resolver expects the query and an arbitrary "requester" object for tracking
const res = await player.search("lofi beats to study to", message.author);

if (res.loadType === 'empty' || !res.tracks.length) {
    return console.log("No tracks found.");
}

if (res.loadType === 'playlist') {
    // Add all tracks to the queue
    player.queue.add(res.tracks);
} else {
    // Add the single track
    player.queue.add(res.tracks[0]);
}
```

---

## Execution & Controls

If the player is currently idle, you must explicitly command it to connect to the voice channel and begin draining the queue.

```javascript
if (!player.playing) {
    await player.connect();
    await player.play();
}
```

### Mutating the Stream

You can mutate the state of the active audio stream using standard asynchronous methods. 

| Method | Description |
| :--- | :--- |
| `await player.pause(boolean)` | Pauses (`true`) or unpauses (`false`) the stream. |
| `await player.resume()` | Resumes a paused stream. |
| `await player.skip()` | Skips to the next track in the queue. |
| `await player.destroy()` | Destroys the C-pointer, clears the queue, and drops the connection. |
| `await player.seek(ms)` | Jumps to a specific millisecond timestamp in the current track. |
| `await player.setVolume(vol)`| Updates the volume (0 to 1000). |

---

## Event Subscriptions

Lavende operates asynchronously and emits lifecycle events via the standard `EventEmitter` interface.

> [!TIP]
> Make sure to call `player.destroy()` on `queueEnd` to free up the allocated memory in the Rust core.

```javascript
player.on('trackStart', (p, track) => {
    console.log(`Now playing: ${track.info.title} requested by ${track.requester.username}`);
});

player.on('trackEnd', (p, track, reason) => {
    // 'reason' can be 'finished', 'stopped', 'replaced', 'loadFailed'
    console.log(`Finished ${track.info.title}`);
});

player.on('queueEnd', (p) => {
    console.log("Queue ended, tearing down session.");
    p.destroy();
});

player.on('error', (p, error) => {
    console.error("A native rust error occurred:", error);
});
```
