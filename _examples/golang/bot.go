package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/disgoorg/disgo"
	"github.com/disgoorg/disgo/bot"
	"github.com/disgoorg/disgo/cache"
	"github.com/disgoorg/disgo/discord"
	"github.com/disgoorg/disgo/events"
	"github.com/disgoorg/disgo/gateway"
	"github.com/disgoorg/snowflake/v2"
	"github.com/joho/godotenv"
	"lavende"
)

var (
	manager *lavende.LavendeManager
)

func formatTime(ms int64) string {
	if ms < 0 {
		return "Live"
	}
	totalSecs := ms / 1000
	hrs := totalSecs / 3600
	mins := (totalSecs % 3600) / 60
	secs := totalSecs % 60
	if hrs > 0 {
		return fmt.Sprintf("%d:%02d:%02d", hrs, mins, secs)
	}
	return fmt.Sprintf("%d:%02d", mins, secs)
}

func onTrackStart(client bot.Client) func(args ...interface{}) {
	return func(args ...interface{}) {
		player := args[0].(*lavende.Player)
		track := args[1].(*lavende.Track)

		embed := discord.NewEmbedBuilder().
			SetTitle("Now Playing").
			SetDescription(fmt.Sprintf("[%s](%s)", track.Info.Title, track.Info.Uri)).
			AddField("Author", track.Info.Author, true).
			AddField("Duration", formatTime(track.Info.Length), true)

		requesterMention := "Unknown"
		if track.Requester != nil {
			if user, ok := track.Requester.(discord.User); ok {
				requesterMention = user.Mention()
			}
		}
		embed.AddField("Requested By", requesterMention, true)

		if track.Info.ArtworkUrl != nil {
			embed.SetThumbnail(*track.Info.ArtworkUrl)
		}

		if player.TextChannelId != nil {
			channelID, _ := snowflake.Parse(*player.TextChannelId)
			_, _ = client.Rest().CreateMessage(channelID, discord.NewMessageCreateBuilder().
				SetEmbeds(embed.Build()).
				Build())
		}
	}
}

func onTrackEnd(client bot.Client) func(args ...interface{}) {
	return func(args ...interface{}) {
		player := args[0].(*lavende.Player)
		track := args[1].(*lavende.Track)
		reason := args[2].(string)

		embed := discord.NewEmbedBuilder().
			SetDescription(fmt.Sprintf("Finished playing: `%s` (Reason: `%s`)", track.Info.Title, reason)).
			Build()

		if player.TextChannelId != nil {
			channelID, _ := snowflake.Parse(*player.TextChannelId)
			_, _ = client.Rest().CreateMessage(channelID, discord.NewMessageCreateBuilder().
				SetEmbeds(embed).
				Build())
		}
	}
}

func onQueueEnd(client bot.Client) func(args ...interface{}) {
	return func(args ...interface{}) {
		player := args[0].(*lavende.Player)

		embed := discord.NewEmbedBuilder().
			SetDescription("Queue ended. Disconnecting from voice channel.").
			Build()

		if player.TextChannelId != nil {
			channelID, _ := snowflake.Parse(*player.TextChannelId)
			_, _ = client.Rest().CreateMessage(channelID, discord.NewMessageCreateBuilder().
				SetEmbeds(embed).
				Build())
		}
		player.Destroy(nil)
	}
}

func onError(client bot.Client) func(args ...interface{}) {
	return func(args ...interface{}) {
		player := args[0].(*lavende.Player)
		err := args[1].(error)

		embed := discord.NewEmbedBuilder().
			SetDescription(fmt.Sprintf("Playback error: `%v`", err)).
			Build()

		if player.TextChannelId != nil {
			channelID, _ := snowflake.Parse(*player.TextChannelId)
			_, _ = client.Rest().CreateMessage(channelID, discord.NewMessageCreateBuilder().
				SetEmbeds(embed).
				Build())
		}
	}
}

func main() {
	// Load environment variables from .env file
	if err := godotenv.Load(); err != nil {
		log.Println("Warning: .env file not found, using environment variables")
	}

	token := os.Getenv("DISCORD_TOKEN")
	if token == "" {
		log.Fatal("DISCORD_TOKEN is missing!")
	}

	var clientRef bot.Client

	client, err := disgo.New(token,
		bot.WithGatewayConfigOpts(gateway.WithIntents(gateway.IntentGuilds|gateway.IntentGuildMessages|gateway.IntentMessageContent|gateway.IntentGuildVoiceStates)),
		bot.WithCacheConfigOpts(
			// Enable all caches including voice states
			cache.WithCaches(cache.FlagsAll),
		),
		bot.WithEventListenerFunc(func(event *events.Ready) {
			log.Printf("Logged in as %s", event.User.Username)

			opts := lavende.LavendeManagerOptions{
				SendToShard: func(guildId string, payload interface{}) {
					log.Printf("SendToShard called for guild %s", guildId)
					// Send voice state updates to Discord Gateway
					// Lavende sends OP 4 (Voice State Update) payloads
					if payloadMap, ok := payload.(map[string]interface{}); ok {
						log.Printf("Payload: %+v", payloadMap)
						if op, ok := payloadMap["op"].(int); ok && op == 4 {
							log.Printf("Processing OP 4 voice state update")
							if d, ok := payloadMap["d"].(map[string]interface{}); ok {
								guildIDSnowflake, _ := snowflake.Parse(d["guild_id"].(string))
								var channelIDPtr *snowflake.ID
								if chID, ok := d["channel_id"].(string); ok && chID != "" {
									chIDSnowflake, _ := snowflake.Parse(chID)
									channelIDPtr = &chIDSnowflake
								}
								selfMute, _ := d["self_mute"].(bool)
								selfDeaf, _ := d["self_deaf"].(bool)
								
								log.Printf("Sending to Discord: guild=%s, channel=%v", guildIDSnowflake, channelIDPtr)
								
								updateData := gateway.MessageDataVoiceStateUpdate{
									GuildID:   guildIDSnowflake,
									ChannelID: channelIDPtr,
									SelfMute:  selfMute,
									SelfDeaf:  selfDeaf,
								}
								
								if err := clientRef.Gateway().Send(context.Background(), gateway.OpcodeVoiceStateUpdate, updateData); err != nil {
									log.Printf("ERROR sending voice update to gateway: %v", err)
								} else {
									log.Printf("Successfully sent OP 4 to Discord")
								}
							}
						}
					}
				},
			}
			opts.Client.Id = event.User.ID.String()
			username := event.User.Username
			opts.Client.Username = &username

			manager = lavende.NewLavendeManager(opts)
			manager.Init(nil)
		}),
		bot.WithEventListenerFunc(func(event *events.GuildVoiceStateUpdate) {
			if manager != nil && event.VoiceState.UserID == clientRef.ApplicationID() {
				log.Printf("Received VOICE_STATE_UPDATE: session_id=%s, channel_id=%v", event.VoiceState.SessionID, event.VoiceState.ChannelID)
				packet := map[string]interface{}{
					"t": "VOICE_STATE_UPDATE",
					"d": map[string]interface{}{
						"user_id":    event.VoiceState.UserID.String(),
						"guild_id":   event.VoiceState.GuildID.String(),
						"session_id": event.VoiceState.SessionID,
						"channel_id": event.VoiceState.ChannelID.String(),
					},
				}
				manager.SendRawData(packet)
				log.Printf("Sent VOICE_STATE_UPDATE to Lavende manager")
			}
		}),
		bot.WithEventListenerFunc(func(event *events.VoiceServerUpdate) {
			if manager != nil {
				log.Printf("Received VOICE_SERVER_UPDATE: endpoint=%s", event.Endpoint)
				packet := map[string]interface{}{
					"t": "VOICE_SERVER_UPDATE",
					"d": map[string]interface{}{
						"guild_id": event.GuildID.String(),
						"token":    event.Token,
						"endpoint": event.Endpoint,
					},
				}
				manager.SendRawData(packet)
				log.Printf("Sent VOICE_SERVER_UPDATE to Lavende manager")
			}
		}),
		bot.WithEventListenerFunc(func(event *events.MessageCreate) {
			if event.Message.Author.Bot || !strings.HasPrefix(event.Message.Content, "!") {
				return
			}
			if event.GuildID == nil {
				return
			}

			content := strings.TrimPrefix(event.Message.Content, "!")
			args := strings.Fields(content)
			if len(args) == 0 {
				return
			}
			command := strings.ToLower(args[0])
			args = args[1:]

			guildID := event.GuildID.String()

			if command == "play" || command == "p" {
				query := strings.Join(args, " ")
				if query == "" {
					embed := discord.NewEmbedBuilder().
						SetDescription("Please provide a track URL or search query.").
						Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().
						SetEmbeds(embed).
						Build())
					return
				}

				// Get user's voice state from cache
				voiceState, ok := clientRef.Caches().VoiceState(*event.GuildID, event.Message.Author.ID)
				if !ok || voiceState.ChannelID == nil {
					embed := discord.NewEmbedBuilder().
						SetDescription("You must be in a voice channel to play music.").
						Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().
						SetEmbeds(embed).
						Build())
					return
				}

				voiceChannelID := voiceState.ChannelID.String()
				log.Printf("User is in voice channel: %s", voiceChannelID)

				var player *lavende.Player
				if pVal, ok := manager.Players.Load(guildID); ok {
					player = pVal.(*lavende.Player)
				} else {
					textChan := event.ChannelID.String()
					sd := true
					player = manager.CreatePlayer(lavende.PlayerOptions{
						GuildId:        guildID,
						VoiceChannelId: voiceChannelID,
						TextChannelId:  &textChan,
						SelfDeaf:       &sd,
					})
					player.On("trackStart", onTrackStart(clientRef))
					player.On("trackEnd", onTrackEnd(clientRef))
					player.On("queueEnd", onQueueEnd(clientRef))
					player.On("error", onError(clientRef))
				}

				resolveEmbed := discord.NewEmbedBuilder().
					SetDescription(fmt.Sprintf("Resolving: `%s`...", query)).
					Build()
				statusMsg, err := clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().
					SetEmbeds(resolveEmbed).
					Build())
				if err != nil {
					return
				}

				res, err := player.Search(query, event.Message.Author)
				if err != nil || res == nil || res.LoadType == "empty" || len(res.Tracks) == 0 {
					embed := discord.NewEmbedBuilder().
						SetDescription("No tracks found.").
						Build()
					_, _ = clientRef.Rest().UpdateMessage(event.ChannelID, statusMsg.ID, discord.NewMessageUpdateBuilder().
						SetEmbeds(embed).
						Build())
					return
				}

				if res.LoadType == "playlist" {
					player.Queue.AddMultiple(res.Tracks, nil)
					playlistName := "Unknown Playlist"
					if infoMap, ok := res.PlaylistInfo.(map[string]interface{}); ok {
						if name, ok := infoMap["name"].(string); ok {
							playlistName = name
						}
					}
					embed := discord.NewEmbedBuilder().
						SetTitle("Playlist Enqueued").
						SetDescription(fmt.Sprintf("Added %d tracks from playlist %s.", len(res.Tracks), playlistName)).
						Build()
					_, _ = clientRef.Rest().UpdateMessage(event.ChannelID, statusMsg.ID, discord.NewMessageUpdateBuilder().
						SetEmbeds(embed).
						Build())
				} else {
					track := res.Tracks[0]
					player.Queue.AddSingle(track, nil)
					embedBuilder := discord.NewEmbedBuilder().
						SetTitle("Track Enqueued").
						SetDescription(fmt.Sprintf("[%s](%s)", track.Info.Title, track.Info.Uri))
					if track.Info.ArtworkUrl != nil {
						embedBuilder.SetThumbnail(*track.Info.ArtworkUrl)
					}
					_, _ = clientRef.Rest().UpdateMessage(event.ChannelID, statusMsg.ID, discord.NewMessageUpdateBuilder().
						SetEmbeds(embedBuilder.Build()).
						Build())
				}

				log.Printf("Player.Playing status: %v, Queue size: %d", player.Playing, player.Queue.Size())
				if !player.Playing {
					log.Printf("Player not playing. Connecting to voice channel %s and starting playback...", voiceChannelID)
					err := player.Connect()
					if err != nil {
						log.Printf("Error connecting to voice: %v", err)
					}
					err = player.Play(nil)
					if err != nil {
						log.Printf("Error starting playback: %v", err)
					}
				} else {
					log.Printf("Player already playing, not reconnecting")
				}

			} else if command == "pause" {
				if pVal, ok := manager.Players.Load(guildID); ok {
					player := pVal.(*lavende.Player)
					player.Pause(true)
					embed := discord.NewEmbedBuilder().SetDescription("Paused.").Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				} else {
					embed := discord.NewEmbedBuilder().SetDescription("No active player.").Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				}

			} else if command == "resume" {
				if pVal, ok := manager.Players.Load(guildID); ok {
					player := pVal.(*lavende.Player)
					player.Resume()
					embed := discord.NewEmbedBuilder().SetDescription("Resumed.").Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				} else {
					embed := discord.NewEmbedBuilder().SetDescription("No active player.").Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				}

			} else if command == "skip" || command == "s" {
				if pVal, ok := manager.Players.Load(guildID); ok {
					player := pVal.(*lavende.Player)
					player.Skip()
					embed := discord.NewEmbedBuilder().SetDescription("Skipped.").Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				} else {
					embed := discord.NewEmbedBuilder().SetDescription("No active player.").Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				}

			} else if command == "stop" {
				if pVal, ok := manager.Players.Load(guildID); ok {
					player := pVal.(*lavende.Player)
					player.Destroy(nil)
					embed := discord.NewEmbedBuilder().SetDescription("Stopped playback and left voice channel.").Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				} else {
					embed := discord.NewEmbedBuilder().SetDescription("No active player.").Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				}

			} else if command == "volume" || command == "vol" {
				if pVal, ok := manager.Players.Load(guildID); ok {
					player := pVal.(*lavende.Player)
					if len(args) == 0 {
						embed := discord.NewEmbedBuilder().SetDescription("Please specify volume value between 0 and 1000.").Build()
						_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
						return
					}
					vol, err := strconv.Atoi(args[0])
					if err != nil || vol < 0 || vol > 1000 {
						embed := discord.NewEmbedBuilder().SetDescription("Please specify a volume value between 0 and 1000.").Build()
						_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
						return
					}
					player.SetVolumeObj(vol)
					embed := discord.NewEmbedBuilder().SetDescription(fmt.Sprintf("Volume set to %d.", vol)).Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				} else {
					embed := discord.NewEmbedBuilder().SetDescription("No active player.").Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				}

			} else if command == "bassboost" || command == "bb" {
				if pVal, ok := manager.Players.Load(guildID); ok {
					player := pVal.(*lavende.Player)
					active := len(player.FilterManager.EqualizerBands) > 0
					if active {
						player.FilterManager.EqualizerBands = []lavende.EqBand{}
						player.FilterManager.ApplyPlayerFilters()
						embed := discord.NewEmbedBuilder().SetDescription("Disabled Bassboost.").Build()
						_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
					} else {
						player.FilterManager.EqualizerBands = []lavende.EqBand{
							{Band: 0, Gain: 0.25},
							{Band: 1, Gain: 0.30},
							{Band: 2, Gain: 0.20},
							{Band: 3, Gain: 0.10},
							{Band: 4, Gain: 0.05},
						}
						player.FilterManager.ApplyPlayerFilters()
						embed := discord.NewEmbedBuilder().SetDescription("Enabled Bassboost.").Build()
						_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
					}
				} else {
					embed := discord.NewEmbedBuilder().SetDescription("No active player.").Build()
					_, _ = clientRef.Rest().CreateMessage(event.ChannelID, discord.NewMessageCreateBuilder().SetEmbeds(embed).Build())
				}
			}
		}),
	)

	if err != nil {
		log.Fatalf("Error creating Discord client: %v", err)
	}

	clientRef = client

	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	if err = client.OpenGateway(ctx); err != nil {
		log.Fatalf("Error opening connection to Discord: %v", err)
	}

	log.Println("Bot is running. Press CTRL-C to exit.")
	sc := make(chan os.Signal, 1)
	signal.Notify(sc, syscall.SIGINT, syscall.SIGTERM, os.Interrupt)
	<-sc

	ctx, cancel = context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	client.Close(ctx)
}


