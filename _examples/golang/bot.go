package main

import (
	"fmt"
	"log"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"

	"github.com/bwmarrin/discordgo"
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

func getVoiceChannelID(s *discordgo.Session, guildID, userID string) string {
	vs, err := s.State.VoiceState(guildID, userID)
	if err == nil && vs != nil {
		return vs.ChannelID
	}
	g, err := s.State.Guild(guildID)
	if err == nil && g != nil {
		for _, state := range g.VoiceStates {
			if state.UserID == userID {
				return state.ChannelID
			}
		}
	}
	return ""
}

func onTrackStart(s *discordgo.Session) func(args ...interface{}) {
	return func(args ...interface{}) {
		player := args[0].(*lavende.Player)
		track := args[1].(*lavende.Track)

		embed := &discordgo.MessageEmbed{
			Title:       "Now Playing",
			Description: fmt.Sprintf("[%s](%s)", track.Info.Title, track.Info.Uri),
		}

		author := track.Info.Author
		if author == "" {
			author = "Unknown"
		}
		embed.Fields = append(embed.Fields, &discordgo.MessageEmbedField{
			Name:   "Author",
			Value:  author,
			Inline: true,
		})

		embed.Fields = append(embed.Fields, &discordgo.MessageEmbedField{
			Name:   "Duration",
			Value:  formatTime(track.Info.Length),
			Inline: true,
		})

		requesterMention := "Unknown"
		if track.Requester != nil {
			if user, ok := track.Requester.(*discordgo.User); ok {
				requesterMention = user.Mention()
			} else if member, ok := track.Requester.(*discordgo.Member); ok {
				requesterMention = member.User.Mention()
			}
		}
		embed.Fields = append(embed.Fields, &discordgo.MessageEmbedField{
			Name:   "Requested By",
			Value:  requesterMention,
			Inline: true,
		})

		if track.Info.ArtworkUrl != nil {
			embed.Thumbnail = &discordgo.MessageEmbedThumbnail{
				URL: *track.Info.ArtworkUrl,
			}
		}

		if player.TextChannelId != nil {
			s.ChannelMessageSendEmbed(*player.TextChannelId, embed)
		}
	}
}

func onTrackEnd(s *discordgo.Session) func(args ...interface{}) {
	return func(args ...interface{}) {
		player := args[0].(*lavende.Player)
		track := args[1].(*lavende.Track)
		reason := args[2].(string)

		embed := &discordgo.MessageEmbed{
			Description: fmt.Sprintf("Finished playing: `%s` (Reason: `%s`)", track.Info.Title, reason),
		}
		if player.TextChannelId != nil {
			s.ChannelMessageSendEmbed(*player.TextChannelId, embed)
		}
	}
}

func onQueueEnd(s *discordgo.Session) func(args ...interface{}) {
	return func(args ...interface{}) {
		player := args[0].(*lavende.Player)

		embed := &discordgo.MessageEmbed{
			Description: "Queue ended. Disconnecting from voice channel.",
		}
		if player.TextChannelId != nil {
			s.ChannelMessageSendEmbed(*player.TextChannelId, embed)
		}
		player.Destroy(nil)
	}
}

func onError(s *discordgo.Session) func(args ...interface{}) {
	return func(args ...interface{}) {
		player := args[0].(*lavende.Player)
		err := args[1].(error)

		embed := &discordgo.MessageEmbed{
			Description: fmt.Sprintf("Playback error: `%v`", err),
		}
		if player.TextChannelId != nil {
			s.ChannelMessageSendEmbed(*player.TextChannelId, embed)
		}
	}
}

func main() {
	token := os.Getenv("DISCORD_TOKEN")
	if token == "" {
		log.Fatal("DISCORD_TOKEN is missing!")
	}

	s, err := discordgo.New("Bot " + token)
	if err != nil {
		log.Fatalf("Error creating Discord session: %v", err)
	}

	s.Identify.Intents = discordgo.IntentsAll

	s.AddHandler(func(s *discordgo.Session, r *discordgo.Ready) {
		log.Printf("Logged in as %s", s.State.User.String())

		opts := lavende.LavendeManagerOptions{
			SendToShard: func(guildId string, payload interface{}) {
				err := s.GatewayWriteStruct(payload)
				if err != nil {
					log.Printf("Error sending raw to shard: %v", err)
				}
			},
		}
		opts.Client.Id = s.State.User.ID
		opts.Client.Username = &s.State.User.Username

		manager = lavende.NewLavendeManager(opts)
		manager.Init(nil)
	})

	s.AddHandler(func(s *discordgo.Session, v *discordgo.VoiceStateUpdate) {
		if manager != nil && v.UserID == s.State.User.ID {
			packet := map[string]interface{}{
				"t": "VOICE_STATE_UPDATE",
				"d": map[string]interface{}{
					"user_id":    v.UserID,
					"guild_id":   v.GuildID,
					"session_id": v.SessionID,
					"channel_id": v.ChannelID,
				},
			}
			manager.SendRawData(packet)
		}
	})

	s.AddHandler(func(s *discordgo.Session, v *discordgo.VoiceServerUpdate) {
		if manager != nil {
			packet := map[string]interface{}{
				"t": "VOICE_SERVER_UPDATE",
				"d": map[string]interface{}{
					"guild_id": v.GuildID,
					"token":    v.Token,
					"endpoint": v.Endpoint,
				},
			}
			manager.SendRawData(packet)
		}
	})

	s.AddHandler(func(s *discordgo.Session, m *discordgo.MessageCreate) {
		if m.Author.Bot || !strings.HasPrefix(m.Content, "!") {
			return
		}
		if m.GuildID == "" {
			return
		}

		content := strings.TrimPrefix(m.Content, "!")
		args := strings.Fields(content)
		if len(args) == 0 {
			return
		}
		command := strings.ToLower(args[0])
		args = args[1:]

		if command == "play" || command == "p" {
			query := strings.Join(args, " ")
			if query == "" {
				embed := &discordgo.MessageEmbed{
					Description: "Please provide a track URL or search query.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
				return
			}

			voiceChannelID := getVoiceChannelID(s, m.GuildID, m.Author.ID)
			if voiceChannelID == "" {
				embed := &discordgo.MessageEmbed{
					Description: "You must be in a voice channel to play music.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
				return
			}

			var player *lavende.Player
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player = pVal.(*lavende.Player)
			} else {
				textChan := m.ChannelID
				sd := true
				player = manager.CreatePlayer(lavende.PlayerOptions{
					GuildId:        m.GuildID,
					VoiceChannelId: voiceChannelID,
					TextChannelId:  &textChan,
					SelfDeaf:       &sd,
				})
				player.On("trackStart", onTrackStart(s))
				player.On("trackEnd", onTrackEnd(s))
				player.On("queueEnd", onQueueEnd(s))
				player.On("error", onError(s))
			}

			resolveEmbed := &discordgo.MessageEmbed{
				Description: fmt.Sprintf("Resolving: `%s`...", query),
			}
			statusMsg, err := s.ChannelMessageSendEmbed(m.ChannelID, resolveEmbed)
			if err != nil {
				return
			}

			res, err := player.Search(query, m.Author)
			if err != nil || res == nil || res.LoadType == "empty" || len(res.Tracks) == 0 {
				embed := &discordgo.MessageEmbed{
					Description: "No tracks found.",
				}
				s.ChannelMessageEditEmbed(m.ChannelID, statusMsg.ID, embed)
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
				embed := &discordgo.MessageEmbed{
					Title:       "Playlist Enqueued",
					Description: fmt.Sprintf("Added %d tracks from playlist %s.", len(res.Tracks), playlistName),
				}
				s.ChannelMessageEditEmbed(m.ChannelID, statusMsg.ID, embed)
			} else {
				track := res.Tracks[0]
				player.Queue.AddSingle(track, nil)
				embed := &discordgo.MessageEmbed{
					Title:       "Track Enqueued",
					Description: fmt.Sprintf("[%s](%s)", track.Info.Title, track.Info.Uri),
				}
				if track.Info.ArtworkUrl != nil {
					embed.Thumbnail = &discordgo.MessageEmbedThumbnail{
						URL: *track.Info.ArtworkUrl,
					}
				}
				s.ChannelMessageEditEmbed(m.ChannelID, statusMsg.ID, embed)
			}

			if !player.Playing {
				player.Connect()
				player.Play(nil)
			}

		} else if command == "pause" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				player.Pause(true)
				embed := &discordgo.MessageEmbed{
					Description: "Paused.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "resume" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				player.Resume()
				embed := &discordgo.MessageEmbed{
					Description: "Resumed.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "skip" || command == "s" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				player.Skip()
				embed := &discordgo.MessageEmbed{
					Description: "Skipped.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "stop" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				player.Destroy(nil)
				embed := &discordgo.MessageEmbed{
					Description: "Stopped playback and left voice channel.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "volume" || command == "vol" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				if len(args) == 0 {
					embed := &discordgo.MessageEmbed{
						Description: "Please specify volume value between 0 and 1000.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
					return
				}
				vol, err := strconv.Atoi(args[0])
				if err != nil || vol < 0 || vol > 1000 {
					embed := &discordgo.MessageEmbed{
						Description: "Please specify a volume value between 0 and 1000.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
					return
				}
				player.SetVolumeObj(vol)
				embed := &discordgo.MessageEmbed{
					Description: fmt.Sprintf("Volume set to %d.", vol),
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "seek" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				if len(args) == 0 {
					embed := &discordgo.MessageEmbed{
						Description: "Please specify seek position in seconds.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
					return
				}
				sec, err := strconv.Atoi(args[0])
				if err != nil || sec < 0 {
					embed := &discordgo.MessageEmbed{
						Description: "Please specify seek position in seconds.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
					return
				}
				player.Seek(int64(sec * 1000))
				embed := &discordgo.MessageEmbed{
					Description: fmt.Sprintf("Seeked to %ds.", sec),
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "repeat" || command == "loop" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				if len(args) == 0 {
					embed := &discordgo.MessageEmbed{
						Description: "Specify repeat mode: off, track, or queue.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
					return
				}
				mode := args[0]
				if mode != "off" && mode != "track" && mode != "queue" {
					embed := &discordgo.MessageEmbed{
						Description: "Specify repeat mode: off, track, or queue.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
					return
				}
				player.SetRepeatMode(mode)
				embed := &discordgo.MessageEmbed{
					Description: fmt.Sprintf("Repeat mode set to %s.", mode),
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "shuffle" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				player.Queue.Shuffle()
				embed := &discordgo.MessageEmbed{
					Description: "Queue shuffled.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "bassboost" || command == "bb" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				active := len(player.FilterManager.EqualizerBands) > 0
				if active {
					player.FilterManager.EqualizerBands = []lavende.EqBand{}
					player.FilterManager.ApplyPlayerFilters()
					embed := &discordgo.MessageEmbed{
						Description: "Disabled Bassboost.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
				} else {
					player.FilterManager.EqualizerBands = []lavende.EqBand{
						{Band: 0, Gain: 0.25},
						{Band: 1, Gain: 0.30},
						{Band: 2, Gain: 0.20},
						{Band: 3, Gain: 0.10},
						{Band: 4, Gain: 0.05},
					}
					player.FilterManager.ApplyPlayerFilters()
					embed := &discordgo.MessageEmbed{
						Description: "Enabled Bassboost.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
				}
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "nightcore" || command == "nc" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				active, _ := player.FilterManager.Filters["nightcore"].(bool)
				if active {
					player.FilterManager.ResetFilters()
					embed := &discordgo.MessageEmbed{
						Description: "Disabled Nightcore filter.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
				} else {
					player.FilterManager.SetSpeed(1.18)
					player.FilterManager.SetPitch(1.3)
					player.FilterManager.Filters["nightcore"] = true
					embed := &discordgo.MessageEmbed{
						Description: "Enabled Nightcore filter.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
				}
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "vaporwave" || command == "vw" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				active, _ := player.FilterManager.Filters["vaporwave"].(bool)
				if active {
					player.FilterManager.ResetFilters()
					embed := &discordgo.MessageEmbed{
						Description: "Disabled Vaporwave filter.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
				} else {
					player.FilterManager.SetSpeed(0.85)
					player.FilterManager.SetPitch(0.8)
					player.FilterManager.Filters["vaporwave"] = true
					embed := &discordgo.MessageEmbed{
						Description: "Enabled Vaporwave filter.",
					}
					s.ChannelMessageSendEmbed(m.ChannelID, embed)
				}
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "rotation" || command == "3d" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				player.FilterManager.ToggleRotation(0.3)
				active, _ := player.FilterManager.Filters["rotation"].(bool)
				desc := "Disabled 3D Rotation filter."
				if active {
					desc = "Enabled 3D Rotation filter."
				}
				embed := &discordgo.MessageEmbed{
					Description: desc,
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "mono" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				player.FilterManager.SetAudioOutput("mono")
				embed := &discordgo.MessageEmbed{
					Description: "Audio output set to Mono.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "stereo" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				player.FilterManager.SetAudioOutput("stereo")
				embed := &discordgo.MessageEmbed{
					Description: "Audio output set to Stereo.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}

		} else if command == "clearfilters" || command == "cf" {
			if pVal, ok := manager.Players.Load(m.GuildID); ok {
				player := pVal.(*lavende.Player)
				player.FilterManager.ResetFilters()
				embed := &discordgo.MessageEmbed{
					Description: "Cleared all active filters.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			} else {
				embed := &discordgo.MessageEmbed{
					Description: "No active player.",
				}
				s.ChannelMessageSendEmbed(m.ChannelID, embed)
			}
		}
	})

	err = s.Open()
	if err != nil {
		log.Fatalf("Error opening connection to Discord: %v", err)
	}

	log.Println("Bot is running. Press CTRL-C to exit.")
	sc := make(chan os.Signal, 1)
	signal.Notify(sc, syscall.SIGINT, syscall.SIGTERM, os.Interrupt)
	<-sc

	s.Close()
}
