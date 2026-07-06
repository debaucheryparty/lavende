require("dotenv").config();
const { Client, GatewayIntentBits, EmbedBuilder } = require("discord.js");
const { LavendeManager } = require("lavende");

const client = new Client({
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildMessages,
    GatewayIntentBits.MessageContent,
    GatewayIntentBits.GuildVoiceStates,
  ],
});

let manager;

function formatTime(ms) {
  if (isNaN(ms) || ms < 0) return "Live";
  const totalSecs = Math.floor(ms / 1000);
  const hrs = Math.floor(totalSecs / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  const secs = totalSecs % 60;
  if (hrs > 0) {
    return `${hrs}:${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

client.once("ready", () => {
  console.log(`Logged in as ${client.user.tag}`);

  manager = new LavendeManager({
    sendToShard: (guildId, payload) => {
      client.guilds.cache.get(guildId)?.shard?.send(payload);
    },
    client: {
      id: client.user.id,
      username: client.user.username,
    },
  });
  manager.init();
});

client.on("raw", async (packet) => {
  if (manager) {
    manager.sendRawData(packet);
  }
});

client.on("messageCreate", async (message) => {
  if (message.author.bot || !message.content.startsWith("!")) return;

  const args = message.content.slice(1).trim().split(/ +/);
  const command = args.shift().toLowerCase();
  const guildId = message.guildId;

  if (command === "play" || command === "p") {
    const query = args.join(" ");
    if (!query) {
      const embed = new EmbedBuilder().setDescription(
        "Please provide a track URL or search query.",
      );
      return message.reply({ embeds: [embed] });
    }

    const voiceChannelId = message.member?.voice?.channelId;
    if (!voiceChannelId) {
      const embed = new EmbedBuilder().setDescription(
        "You must be in a voice channel to play music.",
      );
      return message.reply({ embeds: [embed] });
    }

    let player = manager.players.get(guildId);
    if (!player) {
      player = manager.createPlayer({
        guildId: guildId,
        voiceChannelId: voiceChannelId,
        textChannelId: message.channel.id,
        volume: 100,
      });

      player.on("trackStart", (p, track) => {
        const embed = new EmbedBuilder()
          .setTitle("Now Playing")
          .setDescription(`[${track.info.title}](${track.info.uri})`)
          .addFields(
            {
              name: "Author",
              value: track.info.author || "Unknown",
              inline: true,
            },
            {
              name: "Duration",
              value: formatTime(track.info.length),
              inline: true,
            },
            {
              name: "Requested By",
              value: `<@${track.requester.id}>`,
              inline: true,
            },
          );
        if (track.info.artworkUrl) {
          embed.setThumbnail(track.info.artworkUrl);
        }
        message.channel.send({ embeds: [embed] });
      });

      player.on("trackEnd", (p, track, reason) => {
        const embed = new EmbedBuilder().setDescription(
          `Finished playing: \`${track.info.title}\` (Reason: \`${reason}\`)`,
        );
        message.channel.send({ embeds: [embed] });
      });

      player.on("queueEnd", (p) => {
        const embed = new EmbedBuilder().setDescription(
          "Queue ended. Disconnecting from voice channel.",
        );
        message.channel.send({ embeds: [embed] });
        player.destroy();
      });

      player.on("error", (p, err) => {
        const embed = new EmbedBuilder().setDescription(
          `Playback error: \`${err.message || err}\``,
        );
        message.channel.send({ embeds: [embed] });
      });
    }

    const resolveEmbed = new EmbedBuilder().setDescription(
      `Resolving: \`${query}\`...`,
    );
    const statusMsg = await message.reply({ embeds: [resolveEmbed] });

    try {
      const res = await player.search(query, message.author);
      if (res.loadType === "empty" || !res.tracks.length) {
        const embed = new EmbedBuilder().setDescription("No tracks found.");
        return statusMsg.edit({ embeds: [embed] });
      }

      if (res.loadType === "playlist") {
        player.queue.add(res.tracks);
        const embed = new EmbedBuilder()
          .setTitle("Playlist Enqueued")
          .setDescription(
            `Added ${res.tracks.length} tracks from playlist ${res.playlistInfo?.name}.`,
          );
        statusMsg.edit({ embeds: [embed] });
      } else {
        const track = res.tracks[0];
        player.queue.add(track);
        const embed = new EmbedBuilder()
          .setTitle("Track Enqueued")
          .setDescription(`[${track.info.title}](${track.info.uri})`);
        if (track.info.artworkUrl) {
          embed.setThumbnail(track.info.artworkUrl);
        }
        statusMsg.edit({ embeds: [embed] });
      }

      if (!player.playing) {
        await player.connect();
        await player.play();
      }
    } catch (e) {
      const embed = new EmbedBuilder().setDescription(
        `Search or play error: \`${e.message || e}\``,
      );
      statusMsg.edit({ embeds: [embed] });
      console.error(e);
    }
  } else if (command === "pause") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    await player.pause(true);
    message.reply({ embeds: [new EmbedBuilder().setDescription("Paused.")] });
  } else if (command === "resume") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    await player.resume();
    message.reply({ embeds: [new EmbedBuilder().setDescription("Resumed.")] });
  } else if (command === "skip" || command === "s") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    await player.skip();
    message.reply({ embeds: [new EmbedBuilder().setDescription("Skipped.")] });
  } else if (command === "stop") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    await player.destroy();
    message.reply({
      embeds: [
        new EmbedBuilder().setDescription(
          "Stopped playback and left voice channel.",
        ),
      ],
    });
  } else if (command === "volume" || command === "vol") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    const vol = parseInt(args[0]);
    if (isNaN(vol) || vol < 0 || vol > 1000) {
      return message.reply({
        embeds: [
          new EmbedBuilder().setDescription(
            "Please specify a volume value between 0 and 1000.",
          ),
        ],
      });
    }
    await player.setVolume(vol);
    message.reply({
      embeds: [new EmbedBuilder().setDescription(`Volume set to ${vol}.`)],
    });
  } else if (command === "seek") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    const sec = parseInt(args[0]);
    if (isNaN(sec) || sec < 0) {
      return message.reply({
        embeds: [
          new EmbedBuilder().setDescription(
            "Please specify seek position in seconds.",
          ),
        ],
      });
    }
    await player.seek(sec * 1000);
    message.reply({
      embeds: [new EmbedBuilder().setDescription(`Seeked to ${sec}s.`)],
    });
  } else if (command === "repeat" || command === "loop") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    const mode = args[0];
    if (mode !== "off" && mode !== "track" && mode !== "queue") {
      return message.reply({
        embeds: [
          new EmbedBuilder().setDescription(
            "Specify repeat mode: off, track, or queue.",
          ),
        ],
      });
    }
    player.setRepeatMode(mode);
    message.reply({
      embeds: [
        new EmbedBuilder().setDescription(`Repeat mode set to ${mode}.`),
      ],
    });
  } else if (command === "shuffle") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    player.queue.shuffle();
    message.reply({
      embeds: [new EmbedBuilder().setDescription("Queue shuffled.")],
    });
  } else if (command === "bassboost" || command === "bb") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    const active = player.filterManager.equalizerBands.length > 0;
    if (active) {
      player.filterManager.equalizerBands = [];
      await player.filterManager.applyPlayerFilters();
      message.reply({
        embeds: [new EmbedBuilder().setDescription("Disabled Bassboost.")],
      });
    } else {
      player.filterManager.equalizerBands = [
        { band: 0, gain: 0.25 },
        { band: 1, gain: 0.3 },
        { band: 2, gain: 0.2 },
        { band: 3, gain: 0.1 },
        { band: 4, gain: 0.05 },
      ];
      await player.filterManager.applyPlayerFilters();
      message.reply({
        embeds: [new EmbedBuilder().setDescription("Enabled Bassboost.")],
      });
    }
  } else if (command === "nightcore" || command === "nc") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    const active = player.filterManager.filters.nightcore;
    if (active) {
      await player.filterManager.resetFilters();
      message.reply({
        embeds: [
          new EmbedBuilder().setDescription("Disabled Nightcore filter."),
        ],
      });
    } else {
      await player.filterManager.setSpeed(1.18);
      await player.filterManager.setPitch(1.3);
      player.filterManager.filters.nightcore = true;
      message.reply({
        embeds: [
          new EmbedBuilder().setDescription("Enabled Nightcore filter."),
        ],
      });
    }
  } else if (command === "vaporwave" || command === "vw") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    const active = player.filterManager.filters.vaporwave;
    if (active) {
      await player.filterManager.resetFilters();
      message.reply({
        embeds: [
          new EmbedBuilder().setDescription("Disabled Vaporwave filter."),
        ],
      });
    } else {
      await player.filterManager.setSpeed(0.85);
      await player.filterManager.setPitch(0.8);
      player.filterManager.filters.vaporwave = true;
      message.reply({
        embeds: [
          new EmbedBuilder().setDescription("Enabled Vaporwave filter."),
        ],
      });
    }
  } else if (command === "rotation" || command === "3d") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    await player.filterManager.toggleRotation(0.3);
    const active = player.filterManager.filters.rotation;
    message.reply({
      embeds: [
        new EmbedBuilder().setDescription(
          active
            ? "Enabled 3D Rotation filter."
            : "Disabled 3D Rotation filter.",
        ),
      ],
    });
  } else if (command === "mono") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    await player.filterManager.setAudioOutput("mono");
    message.reply({
      embeds: [new EmbedBuilder().setDescription("Audio output set to Mono.")],
    });
  } else if (command === "stereo") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    await player.filterManager.setAudioOutput("stereo");
    message.reply({
      embeds: [
        new EmbedBuilder().setDescription("Audio output set to Stereo."),
      ],
    });
  } else if (command === "clearfilters" || command === "cf") {
    const player = manager.players.get(guildId);
    if (!player)
      return message.reply({
        embeds: [new EmbedBuilder().setDescription("No active player.")],
      });
    await player.filterManager.resetFilters();
    message.reply({
      embeds: [
        new EmbedBuilder().setDescription("Cleared all active filters."),
      ],
    });
  }
});

if (!process.env.DISCORD_TOKEN) {
  console.error("DISCORD_TOKEN is not set in the environment.");
  process.exit(1);
}
client.login(process.env.DISCORD_TOKEN);
