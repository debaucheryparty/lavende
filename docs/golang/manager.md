# The Golang Manager

The `LavendeManager` acts as the orchestrator for your audio sessions in Go. It connects your Go code to the internal Rust core via CGO.

---

## Bootstrapping the Manager

Because Lavende leverages standard Discord voice protocol principles, you must provide it with a mechanism to send payloads back to Discord's Voice Gateway.

> [!IMPORTANT]
> Initialize the manager once, globally, after your Discord bot is ready. Lavende works with **Disgo** (not discordgo).

### Initialization Parameters

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `SendToShard` | `func(guildId string, payload interface{})` | Yes | A function mapping a Guild ID to the corresponding WebSocket shard for writing JSON payloads. |
| `Client.Id` | `string` | Yes | Your Bot's Application ID. |
| `Client.Username` | `*string` | Yes | Your Bot's Username. |

### Example Setup (Disgo)

```go
import (
    "context"
    "github.com/disgoorg/disgo"
    "github.com/disgoorg/disgo/bot"
    "github.com/disgoorg/disgo/events"
    "github.com/disgoorg/disgo/gateway"
    "lavende"
)

var manager *lavende.LavendeManager

func main() {
    token := os.Getenv("DISCORD_TOKEN")
    
    client, err := disgo.New(token,
        bot.WithGatewayConfigOpts(
            gateway.WithIntents(
                gateway.IntentGuilds |
                gateway.IntentGuildMessages |
                gateway.IntentMessageContent |
                gateway.IntentGuildVoiceStates,
            ),
        ),
        bot.WithEventListenerFunc(onReady),
        bot.WithEventListenerFunc(onVoiceStateUpdate),
        bot.WithEventListenerFunc(onVoiceServerUpdate),
    )
    
    if err != nil {
        log.Fatal(err)
    }
    
    ctx := context.Background()
    client.OpenGateway(ctx)
}

func onReady(event *events.Ready) {
    log.Printf("Logged in as %s", event.User.Username)
    
    opts := lavende.LavendeManagerOptions{
        SendToShard: func(guildId string, payload interface{}) {
            // Disgo automatically handles sending to the gateway
            _ = client.Gateway().Send(context.Background(), payload)
        },
    }
    opts.Client.Id = event.User.ID.String()
    username := event.User.Username
    opts.Client.Username = &username
    
    manager = lavende.NewLavendeManager(opts)
    manager.Init(nil)
    
    log.Println("Lavende Manager Initialized")
}
```

---

## Intercepting Gateway Events

Lavende requires `VOICE_STATE_UPDATE` and `VOICE_SERVER_UPDATE` Discord Gateway events to negotiate the secure UDP voice connection.

You must listen for these events via Disgo and pipe the raw struct equivalents into the manager.

```go
func onVoiceStateUpdate(event *events.VoiceStateUpdate) {
    // Only forward if the voice state update is for our bot!
    if manager != nil && event.VoiceState.UserID == client.ApplicationID() {
        manager.SendRawData(map[string]interface{}{
            "t": "VOICE_STATE_UPDATE",
            "d": map[string]interface{}{
                "user_id":    event.VoiceState.UserID.String(),
                "guild_id":   event.VoiceState.GuildID.String(),
                "session_id": event.VoiceState.SessionID,
                "channel_id": event.VoiceState.ChannelID.String(),
            },
        })
    }
}

func onVoiceServerUpdate(event *events.VoiceServerUpdate) {
    if manager != nil {
        manager.SendRawData(map[string]interface{}{
            "t": "VOICE_SERVER_UPDATE",
            "d": map[string]interface{}{
                "guild_id": event.GuildID.String(),
                "token":    event.Token,
                "endpoint": event.Endpoint,
            },
        })
    }
}
```

> [!NOTE]
> Once the Rust core receives both the state update and the server update, it will independently establish the UDP socket and perform the secret key exchange, freeing up your Go goroutines.
