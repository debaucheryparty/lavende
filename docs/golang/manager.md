# The Golang Manager

The `LavendeManager` acts as the orchestrator for your audio sessions in Go. It connects your Go code to the internal Rust core via CGO.

---

## Bootstrapping the Manager

Because Lavende leverages standard Discord voice protocol principles, you must provide it with a mechanism to send payloads back to Discord's Voice Gateway.

> [!IMPORTANT]
> Initialize the manager once, globally, before your Discord bot starts processing commands.

### Initialization Parameters

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `SendToShard` | `func(guildId string, payload interface{})` | Yes | A function mapping a Guild ID to the corresponding WebSocket shard for writing JSON payloads. |
| `Client.Id` | `string` | Yes | Your Bot's Application ID. |
| `Client.Username` | `*string` | Yes | Your Bot's Username. |

### Example Setup (discordgo)

```go
import (
    "github.com/bwmarrin/discordgo"
    "lavende"
)

var manager *lavende.LavendeManager

func initAudio(s *discordgo.Session) {
    opts := lavende.LavendeManagerOptions{
        SendToShard: func(guildId string, payload interface{}) {
            // Write payload to websocket for the specific shard handling this guild
            _ = s.GatewayWriteStruct(payload)
        },
    }
    opts.Client.Id = s.State.User.ID
    opts.Client.Username = &s.State.User.Username

    manager = lavende.NewLavendeManager(opts)
    manager.Init(nil)
    
    fmt.Println("Lavende Manager Initialized")
}
```

---

## Intercepting Gateway Events

Lavende requires `VOICE_STATE_UPDATE` and `VOICE_SERVER_UPDATE` Discord Gateway events to negotiate the secure UDP voice connection.

You must listen for these events via your Discord library (e.g., `discordgo`) and pipe the raw struct equivalents into the manager.

```go
s.AddHandler(func(s *discordgo.Session, v *discordgo.VoiceStateUpdate) {
    // Only forward if the voice state update is for our bot!
    if manager != nil && v.UserID == s.State.User.ID {
        manager.SendRawData(map[string]interface{}{
            "t": "VOICE_STATE_UPDATE",
            "d": map[string]interface{}{
                "user_id":    v.UserID,
                "guild_id":   v.GuildID,
                "session_id": v.SessionID,
                "channel_id": v.ChannelID,
            },
        })
    }
})

s.AddHandler(func(s *discordgo.Session, v *discordgo.VoiceServerUpdate) {
    if manager != nil {
        manager.SendRawData(map[string]interface{}{
            "t": "VOICE_SERVER_UPDATE",
            "d": map[string]interface{}{
                "guild_id": v.GuildID,
                "token":    v.Token,
                "endpoint": v.Endpoint,
            },
        })
    }
})
```

> [!NOTE]
> Once the Rust core receives both the state update and the server update, it will independently establish the UDP socket and perform the secret key exchange, freeing up your Go goroutines.
