# Players & Queues (Golang)

A `Player` encapsulates the audio session and queue for a specific Discord Guild. 

---

## Player Instantiation

When a user initiates an audio command, you should check the `manager.Players` `sync.Map` for an active session. If it doesn't exist, allocate one.

```go
var player *lavende.Player

if pVal, ok := manager.Players.Load(guildID); ok {
    player = pVal.(*lavende.Player)
} else {
    sd := true
    player = manager.CreatePlayer(lavende.PlayerOptions{
        GuildId:        guildID,
        VoiceChannelId: voiceChannelID,
        TextChannelId:  &textChannelID,
        SelfDeaf:       &sd,
        Volume:         100, // 0 to 1000 percentage
    })
}
```

---

## Audio Resolution

The resolver queries external APIs and parses metadata. It returns a structured `ResolveResponse`.

| `LoadType` | Description |
| :--- | :--- |
| `"empty"` | The query yielded no results. |
| `"playlist"` | A collection of tracks was returned. |
| `"track"` | A single track or search result was returned. |

```go
res, err := player.Search("query or URL", requester)

if err != nil || res == nil || res.LoadType == "empty" {
    fmt.Println("No tracks found.")
    return
}

if res.LoadType == "playlist" {
    player.Queue.AddMultiple(res.Tracks, nil)
} else {
    player.Queue.AddSingle(res.Tracks[0], nil)
}
```

---

## Execution & Controls

If the player is currently idle, you must explicitly connect and start playback.

```go
if !player.Playing {
    player.Connect()
    player.Play(nil)
}
```

### Mutating the Stream

Control methods safely cross the CGO boundary to manipulate the internal C state in Rust.

| Method | Description |
| :--- | :--- |
| `player.Pause(bool)` | Pauses (`true`) or unpauses (`false`) the stream. |
| `player.Resume()` | Resumes a paused stream. |
| `player.Skip()` | Skips to the next track in the queue. |
| `player.Destroy(nil)` | Clears the queue, drops the connection, and frees C memory. |
| `player.Seek(ms int)` | Jumps to a specific millisecond timestamp. |
| `player.SetVolumeObj(int)`| Updates the volume (0 to 1000). |

---

## Event Subscriptions

The Golang event dispatcher dynamically casts arguments. You must define handlers that explicitly type-cast `interface{}` to the known pointer types (`*lavende.Player`, `*lavende.Track`).

> [!TIP]
> Ensure you invoke `player.Destroy(nil)` on `queueEnd` to prevent memory leaks in the C heap.

```go
func onTrackStart(args ...interface{}) {
    player := args[0].(*lavende.Player)
    track := args[1].(*lavende.Track)
    fmt.Printf("Started playing: %s\n", track.Info.Title)
}

func onTrackEnd(args ...interface{}) {
    player := args[0].(*lavende.Player)
    track := args[1].(*lavende.Track)
    reason := args[2].(string) // 'finished', 'stopped', 'replaced'
    fmt.Printf("Finished playing %s. Reason: %s\n", track.Info.Title, reason)
}

func onQueueEnd(args ...interface{}) {
    player := args[0].(*lavende.Player)
    fmt.Println("Queue ended, cleaning up memory.")
    player.Destroy(nil)
}

// Bind handlers
player.On("trackStart", onTrackStart)
player.On("trackEnd", onTrackEnd)
player.On("queueEnd", onQueueEnd)
```
