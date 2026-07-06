import discord
from discord.ext import commands
import os
import sys
import json
import asyncio
from dotenv import load_dotenv
from lavende import LavendeManager

# Load environment variables
load_dotenv()

intents = discord.Intents.all()

bot = commands.Bot(command_prefix="!", intents=intents, enable_debug_events=True)
manager = None


def format_time(ms):
    if ms is None or ms < 0:
        return "Live"
    total_secs = int(ms // 1000)
    hrs = total_secs // 3600
    mins = (total_secs % 3600) // 60
    secs = total_secs % 60
    if hrs > 0:
        return f"{hrs}:{mins:02d}:{secs:02d}"
    return f"{mins}:{secs:02d}"


async def send_to_shard(guild_id: str, payload: dict):
    guild = bot.get_guild(int(guild_id))
    shard_id = guild.shard_id if guild else 0
    ws = bot.shards.get(shard_id) if hasattr(bot, "shards") else None
    if not ws:
        ws = bot.ws
    if ws:
        await ws.send_as_json(payload)


async def get_text_channel(player):
    if not player.text_channel_id:
        return None
    try:
        channel = bot.get_channel(int(player.text_channel_id))
        if not channel:
            channel = await bot.fetch_channel(int(player.text_channel_id))
        return channel
    except Exception:
        return None


async def on_track_start(p, track):
    embed = discord.Embed(
        title="Now Playing",
        description=(
            f"[{track.info.title}]({track.info.uri})"
            if track.info.uri
            else track.info.title
        ),
    )
    embed.add_field(name="Author", value=track.info.author or "Unknown", inline=True)
    embed.add_field(name="Duration", value=format_time(track.info.length), inline=True)

    requester_mention = "Unknown"
    if track.requester:
        if hasattr(track.requester, "mention"):
            requester_mention = track.requester.mention
        else:
            requester_mention = f"<@{track.requester}>"

    embed.add_field(name="Requested By", value=requester_mention, inline=True)

    if track.info.artwork_url:
        embed.set_thumbnail(url=track.info.artwork_url)

    channel = await get_text_channel(p)
    if channel:
        await channel.send(embed=embed)


async def on_track_end(p, track, reason):
    embed = discord.Embed(
        description=f"Finished playing: `{track.info.title}` (Reason: `{reason}`)"
    )
    channel = await get_text_channel(p)
    if channel:
        await channel.send(embed=embed)


async def on_queue_end(p):
    embed = discord.Embed(description="Queue ended. Disconnecting from voice channel.")
    channel = await get_text_channel(p)
    if channel:
        await channel.send(embed=embed)
    await p.destroy()


async def on_error(p, err):
    err_msg = getattr(err, "message", str(err))
    embed = discord.Embed(description=f"Playback error: `{err_msg}`")
    channel = await get_text_channel(p)
    if channel:
        await channel.send(embed=embed)


@bot.event
async def on_ready():
    global manager
    print(f"Logged in as {bot.user.name} ({bot.user.id})")

    manager = LavendeManager(
        send_to_shard=send_to_shard,
        client={"id": str(bot.user.id), "username": bot.user.name},
    )
    manager.init()


@bot.event
async def on_socket_raw_receive(msg):
    global manager
    if manager:
        try:
            if isinstance(msg, str):
                packet = json.loads(msg)
            elif isinstance(msg, bytes):
                packet = json.loads(msg.decode("utf-8"))
            else:
                packet = msg

            await manager.send_raw_data(packet)
        except Exception:
            pass


@bot.event
async def on_message(message):
    if message.author.bot or not message.content.startswith("!"):
        return

    if not message.guild:
        return

    args = message.content[1:].strip().split()
    if not args:
        return

    command = args.pop(0).lower()
    guild_id = str(message.guild.id)

    if command in ("play", "p"):
        query = " ".join(args)
        if not query:
            embed = discord.Embed(
                description="Please provide a track URL or search query."
            )
            return await message.reply(embed=embed)

        voice_state = message.author.voice
        if not voice_state or not voice_state.channel:
            embed = discord.Embed(
                description="You must be in a voice channel to play music."
            )
            return await message.reply(embed=embed)

        voice_channel_id = str(voice_state.channel.id)

        player = manager.players.get(guild_id)
        if not player:
            player = manager.create_player(
                {
                    "guild_id": guild_id,
                    "voice_channel_id": voice_channel_id,
                    "text_channel_id": str(message.channel.id),
                    "volume": 100,
                }
            )

            player.on("track_start", on_track_start)
            player.on("track_end", on_track_end)
            player.on("queue_end", on_queue_end)
            player.on("error", on_error)

        resolve_embed = discord.Embed(description=f"Resolving: `{query}`...")
        status_msg = await message.reply(embed=resolve_embed)

        try:
            res = await player.search(query, message.author)
            load_type = res.get("loadType", "empty")
            tracks = res.get("tracks", [])

            if load_type == "empty" or not tracks:
                embed = discord.Embed(description="No tracks found.")
                return await status_msg.edit(embed=embed)

            if load_type == "playlist":
                player.queue.add(tracks)
                playlist_name = res.get("playlistInfo", {}).get(
                    "name", "Unknown Playlist"
                )
                embed = discord.Embed(
                    title="Playlist Enqueued",
                    description=f"Added {len(tracks)} tracks from playlist {playlist_name}.",
                )
                await status_msg.edit(embed=embed)
            else:
                track = tracks[0]
                player.queue.add(track)
                embed = discord.Embed(
                    title="Track Enqueued",
                    description=(
                        f"[{track.info.title}]({track.info.uri})"
                        if track.info.uri
                        else track.info.title
                    ),
                )
                if track.info.artwork_url:
                    embed.set_thumbnail(url=track.info.artwork_url)
                await status_msg.edit(embed=embed)

            if not player.playing:
                await player.connect()
                await player.play()
        except Exception as e:
            err_msg = getattr(e, "message", str(e))
            embed = discord.Embed(description=f"Search or play error: `{err_msg}`")
            await status_msg.edit(embed=embed)
            import traceback

            traceback.print_exc()

    elif command == "pause":
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)
        await player.pause(True)
        embed = discord.Embed(description="Paused.")
        await message.reply(embed=embed)

    elif command == "resume":
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)
        await player.resume()
        embed = discord.Embed(description="Resumed.")
        await message.reply(embed=embed)

    elif command in ("skip", "s"):
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)
        await player.skip()
        embed = discord.Embed(description="Skipped.")
        await message.reply(embed=embed)

    elif command == "stop":
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)
        await player.destroy()
        embed = discord.Embed(description="Stopped playback and left voice channel.")
        await message.reply(embed=embed)

    elif command in ("volume", "vol"):
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)

        try:
            vol = int(args[0])
        except (IndexError, ValueError):
            vol = -1

        if vol < 0 or vol > 1000:
            embed = discord.Embed(
                description="Please specify a volume value between 0 and 1000."
            )
            return await message.reply(embed=embed)

        await player.set_volume(vol)
        embed = discord.Embed(description=f"Volume set to {vol}.")
        await message.reply(embed=embed)

    elif command == "seek":
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)

        try:
            sec = int(args[0])
        except (IndexError, ValueError):
            sec = -1

        if sec < 0:
            embed = discord.Embed(
                description="Please specify seek position in seconds."
            )
            return await message.reply(embed=embed)

        await player.seek(sec * 1000)
        embed = discord.Embed(description=f"Seeked to {sec}s.")
        await message.reply(embed=embed)

    elif command in ("repeat", "loop"):
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)

        mode = args[0] if len(args) > 0 else ""
        if mode not in ("off", "track", "queue"):
            embed = discord.Embed(
                description="Specify repeat mode: off, track, or queue."
            )
            return await message.reply(embed=embed)

        player.set_repeat_mode(mode)
        embed = discord.Embed(description=f"Repeat mode set to {mode}.")
        await message.reply(embed=embed)

    elif command == "shuffle":
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)
        player.queue.shuffle()
        embed = discord.Embed(description="Queue shuffled.")
        await message.reply(embed=embed)

    elif command in ("bassboost", "bb"):
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)

        active = len(player.filter_manager.equalizer_bands) > 0
        if active:
            player.filter_manager.equalizer_bands = []
            await player.filter_manager.apply_player_filters()
            embed = discord.Embed(description="Disabled Bassboost.")
            await message.reply(embed=embed)
        else:
            player.filter_manager.equalizer_bands = [
                {"band": 0, "gain": 0.25},
                {"band": 1, "gain": 0.30},
                {"band": 2, "gain": 0.20},
                {"band": 3, "gain": 0.10},
                {"band": 4, "gain": 0.05},
            ]
            await player.filter_manager.apply_player_filters()
            embed = discord.Embed(description="Enabled Bassboost.")
            await message.reply(embed=embed)

    elif command in ("nightcore", "nc"):
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)

        active = player.filter_manager.filters.get("nightcore", False)
        if active:
            await player.filter_manager.reset_filters()
            embed = discord.Embed(description="Disabled Nightcore filter.")
            await message.reply(embed=embed)
        else:
            await player.filter_manager.set_speed(1.18)
            await player.filter_manager.set_pitch(1.3)
            player.filter_manager.filters["nightcore"] = True
            embed = discord.Embed(description="Enabled Nightcore filter.")
            await message.reply(embed=embed)

    elif command in ("vaporwave", "vw"):
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)

        active = player.filter_manager.filters.get("vaporwave", False)
        if active:
            await player.filter_manager.reset_filters()
            embed = discord.Embed(description="Disabled Vaporwave filter.")
            await message.reply(embed=embed)
        else:
            await player.filter_manager.set_speed(0.85)
            await player.filter_manager.set_pitch(0.8)
            player.filter_manager.filters["vaporwave"] = True
            embed = discord.Embed(description="Enabled Vaporwave filter.")
            await message.reply(embed=embed)

    elif command in ("rotation", "3d"):
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)

        await player.filter_manager.toggle_rotation(0.3)
        active = player.filter_manager.filters.get("rotation", False)
        desc = (
            "Enabled 3D Rotation filter." if active else "Disabled 3D Rotation filter."
        )
        embed = discord.Embed(description=desc)
        await message.reply(embed=embed)

    elif command == "mono":
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)
        await player.filter_manager.set_audio_output("mono")
        embed = discord.Embed(description="Audio output set to Mono.")
        await message.reply(embed=embed)

    elif command == "stereo":
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)
        await player.filter_manager.set_audio_output("stereo")
        embed = discord.Embed(description="Audio output set to Stereo.")
        await message.reply(embed=embed)

    elif command in ("clearfilters", "cf"):
        player = manager.players.get(guild_id)
        if not player:
            embed = discord.Embed(description="No active player.")
            return await message.reply(embed=embed)
        await player.filter_manager.reset_filters()
        embed = discord.Embed(description="Cleared all active filters.")
        await message.reply(embed=embed)


if __name__ == "__main__":
    token = os.getenv("DISCORD_TOKEN")
    if not token:
        print("DISCORD_TOKEN is not set in the environment.")
        sys.exit(1)

    bot.run(token)
