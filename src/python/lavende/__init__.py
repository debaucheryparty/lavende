import asyncio
import json
import random
from typing import Any, Callable, Dict, List, Literal, Optional, Union

from lavende._lavende import Player as _RustPlayer
from lavende._lavende import load as _rust_load
from lavende.constants import (
    DebugEvents,
    DestroyReasons,
    DisconnectReasons,
    VALID_SPONSOR_BLOCKS,
    AUDIO_OUTPUTS_DATA,
    EQ_LIST,
    DEFAULT_SOURCES,
    SOURCE_LINKS_REGEXES,
)

__version__ = "0.1.14"


class TrackInfo:
    def __init__(self, data: dict):
        self.title: str = data.get("title", "")
        self.author: str = data.get("author", "")
        self.length: int = data.get("length", 0)
        self.identifier: str = data.get("identifier", "")
        self.is_stream: bool = data.get("isStream", False)
        self.is_seekable: bool = data.get("isSeekable", True)
        self.uri: Optional[str] = data.get("uri")
        self.source_name: str = data.get("sourceName", "")
        self.artwork_url: Optional[str] = data.get("artworkUrl")
        self.isrc: Optional[str] = data.get("isrc")
        self.position: int = data.get("position", 0)

    def __repr__(self) -> str:
        return f"TrackInfo(title='{self.title}', author='{self.author}')"


class Track:
    def __init__(self, data: dict, requester=None):
        self.encoded: str = data.get("encoded", "")
        self.info: TrackInfo = TrackInfo(data.get("info", {}))
        self.requester = requester

    def __repr__(self) -> str:
        return f"Track(title='{self.info.title}', author='{self.info.author}')"


class QueueUtils:
    def __init__(self, queue: "Queue"):
        self.queue = queue

    def to_json(self) -> dict:
        return {
            "current": self.queue.current.__dict__ if self.queue.current else None,
            "previous": [t.__dict__ for t in self.queue.previous],
            "tracks": [t.__dict__ for t in self.queue.tracks],
        }

    def total_duration(self) -> int:
        total = sum(t.info.length or 0 for t in self.queue.tracks)
        if self.queue.current:
            total += self.queue.current.info.length or 0
        return total

    def filter_tracks(self, predicate: Callable[[Track, int], bool]) -> List[dict]:
        return [
            {"track": track, "index": idx}
            for idx, track in enumerate(self.queue.tracks)
            if predicate(track, idx)
        ]

    def find_track(self, predicate: Callable[[Track, int], bool]) -> Optional[dict]:
        results = self.filter_tracks(predicate)
        return results[0] if results else None


class Queue:
    def __init__(self, guild_id: str):
        self.tracks: List[Track] = []
        self.current: Optional[Track] = None
        self.previous: List[Track] = []
        self.guild_id: str = guild_id
        self.utils = QueueUtils(self)

    @property
    def size(self) -> int:
        return len(self.tracks)

    def add(
        self, track: Union[Track, List[Track]], index: Optional[int] = None
    ) -> None:
        tracks = track if isinstance(track, list) else [track]
        if index is not None:
            for i, t in enumerate(tracks):
                self.tracks.insert(index + i, t)
        else:
            self.tracks.extend(tracks)

    def remove(self, index: int) -> None:
        if 0 <= index < len(self.tracks):
            self.tracks.pop(index)

    def clear(self) -> None:
        self.tracks = []
        self.current = None
        self.previous = []

    def shuffle(self) -> None:
        random.shuffle(self.tracks)

    def __repr__(self) -> str:
        return f"Queue(guild_id='{self.guild_id}', size={self.size})"


class FilterManager:
    def __init__(self, player: "Player"):
        self.player = player
        self.data: Dict[str, Any] = {}
        self.equalizer_bands: List[Dict[str, float]] = []
        self.filters: Dict[str, Any] = {
            "volume": False,
            "rotation": False,
            "tremolo": False,
            "vibrato": False,
            "low_pass": False,
            "audio_output": "stereo",
        }

    async def apply_player_filters(self) -> None:
        if self.equalizer_bands:
            self.data["equalizer"] = self.equalizer_bands
        elif "equalizer" in self.data:
            del self.data["equalizer"]
        await self.player.set_filters(self.data)

    async def reset_filters(self) -> "FilterManager":
        self.data = {}
        self.equalizer_bands = []
        self.filters = {
            "volume": False,
            "rotation": False,
            "tremolo": False,
            "vibrato": False,
            "low_pass": False,
            "audio_output": "stereo",
        }
        await self.apply_player_filters()
        return self

    async def set_volume(self, volume: float) -> "FilterManager":
        self.data["volume"] = volume
        self.filters["volume"] = volume != 1.0
        await self.apply_player_filters()
        return self

    async def set_audio_output(
        self, output_type: Literal["mono", "stereo", "left", "right"]
    ) -> "FilterManager":
        channel_mix = {
            "mono": {
                "leftToLeft": 0.5,
                "leftToRight": 0.5,
                "rightToLeft": 0.5,
                "rightToRight": 0.5,
            },
            "left": {
                "leftToLeft": 1,
                "leftToRight": 0,
                "rightToLeft": 1,
                "rightToRight": 0,
            },
            "right": {
                "leftToLeft": 0,
                "leftToRight": 1,
                "rightToLeft": 0,
                "rightToRight": 1,
            },
            "stereo": {
                "leftToLeft": 1,
                "leftToRight": 0,
                "rightToLeft": 0,
                "rightToRight": 1,
            },
        }
        self.data["channelMix"] = channel_mix[output_type]
        self.filters["audio_output"] = output_type
        await self.apply_player_filters()
        return self

    async def set_speed(self, speed: float = 1.0) -> "FilterManager":
        self.data.setdefault("timescale", {})["speed"] = speed
        await self.apply_player_filters()
        return self

    async def set_pitch(self, pitch: float = 1.0) -> "FilterManager":
        self.data.setdefault("timescale", {})["pitch"] = pitch
        await self.apply_player_filters()
        return self

    async def set_rate(self, rate: float = 1.0) -> "FilterManager":
        self.data.setdefault("timescale", {})["rate"] = rate
        await self.apply_player_filters()
        return self

    async def toggle_rotation(self, rotation_hz: float = 0.2) -> "FilterManager":
        if self.filters["rotation"]:
            self.data.pop("rotation", None)
        else:
            self.data["rotation"] = {"rotationHz": rotation_hz}
        self.filters["rotation"] = not self.filters["rotation"]
        await self.apply_player_filters()
        return self

    async def toggle_vibrato(
        self, frequency: float = 10.0, depth: float = 1.0
    ) -> "FilterManager":
        if self.filters["vibrato"]:
            self.data.pop("vibrato", None)
        else:
            self.data["vibrato"] = {"frequency": frequency, "depth": depth}
        self.filters["vibrato"] = not self.filters["vibrato"]
        await self.apply_player_filters()
        return self

    async def toggle_tremolo(
        self, frequency: float = 4.0, depth: float = 0.8
    ) -> "FilterManager":
        if self.filters["tremolo"]:
            self.data.pop("tremolo", None)
        else:
            self.data["tremolo"] = {"frequency": frequency, "depth": depth}
        self.filters["tremolo"] = not self.filters["tremolo"]
        await self.apply_player_filters()
        return self


class Player:
    def __init__(self, manager: "LavendeManager", options: Dict[str, Any]):
        self.manager = manager
        self.guild_id: str = options["guild_id"]
        self.voice_channel_id: Optional[str] = options["voice_channel_id"]
        self.text_channel_id: Optional[str] = options.get("text_channel_id")
        self.volume: int = options.get("volume", 100)
        self.self_deaf: bool = options.get("self_deaf", True)

        self.paused: bool = False
        self.playing: bool = False
        self.repeat_mode: Literal["off", "track", "queue"] = "off"
        self.voice_state: Dict[str, Optional[str]] = {
            "session_id": None,
            "token": None,
            "endpoint": None,
        }
        self.node = {
            "session_id": "local-session",
            "_check_for_sources": False,
            "_check_for_plugins": False,
        }
        self.play_on_connect: bool = False
        self._data: Dict[str, Any] = {}
        self._event_handlers: Dict[str, list] = {}

        self._rust_player = _RustPlayer(self.guild_id)
        self.queue = Queue(self.guild_id)
        self.filter_manager = FilterManager(self)

    def on(self, event: str, handler: Callable) -> None:
        self._event_handlers.setdefault(event, []).append(handler)

    def emit(self, event: str, *args, **kwargs) -> None:
        for handler in self._event_handlers.get(event, []):
            if asyncio.iscoroutinefunction(handler):
                asyncio.create_task(handler(*args, **kwargs))
            else:
                handler(*args, **kwargs)

    def set(self, key: str, value: Any) -> "Player":
        self._data[key] = value
        return self

    def get(self, key: str, default: Any = None) -> Any:
        return self._data.get(key, default)

    def set_data(self, key: str, value: Any) -> "Player":
        return self.set(key, value)

    def get_data(self, key: str, default: Any = None) -> Any:
        return self.get(key, default)

    def delete_data(self, key: str) -> "Player":
        self._data.pop(key, None)
        return self

    def clear_data(self) -> "Player":
        self._data = {}
        return self

    def get_all_data(self) -> Dict[str, Any]:
        return self._data.copy()

    def set_voice_state(self, state: Dict[str, Optional[str]]) -> None:
        self.voice_state.update(state)

    def check_play_on_connect(self) -> None:
        s = self.voice_state
        if (
            s.get("session_id")
            and s.get("token")
            and s.get("endpoint")
            and self.play_on_connect
        ):
            self.play_on_connect = False
            print(
                f"[Player {self.guild_id}] Delayed play handshake completed, starting playback."
            )
            asyncio.create_task(self._safe_play())

    async def _safe_play(self):
        try:
            await self.play()
        except Exception as e:
            self.emit("error", self, e)

    async def connect(self) -> None:
        await self.manager.send_to_shard(
            self.guild_id,
            {
                "op": 4,
                "d": {
                    "guild_id": self.guild_id,
                    "channel_id": self.voice_channel_id,
                    "self_mute": False,
                    "self_deaf": self.self_deaf,
                },
            },
        )

    async def disconnect(self) -> None:
        self.voice_channel_id = None
        await self.manager.send_to_shard(
            self.guild_id,
            {
                "op": 4,
                "d": {
                    "guild_id": self.guild_id,
                    "channel_id": None,
                    "self_mute": False,
                    "self_deaf": False,
                },
            },
        )
        await self.stop()

    async def destroy(self, reason: Optional[str] = None) -> None:
        await self.disconnect()
        self.emit("player_destroy", self, reason)
        self.manager.players.pop(self.guild_id, None)

    async def search(
        self, query: Union[str, Dict[str, str]], requester: Any = None
    ) -> Dict[str, Any]:
        search_str = query if isinstance(query, str) else query.get("query", "")
        return await load(search_str, requester)

    async def play(self, options: Optional[Dict[str, Any]] = None) -> None:
        if options is None:
            options = {}
        if "track" in options:
            self.queue.current = options["track"]
        if "volume" in options:
            self.volume = options["volume"]
        if "paused" in options:
            self.paused = options["paused"]

        if not self.queue.current:
            if not self.queue.tracks:
                self.playing = False
                self.emit("queue_end", self)
                return
            self.queue.current = self.queue.tracks.pop(0)

        current_track = self.queue.current
        if not current_track:
            raise ValueError("No track is currently available to play.")

        session_id = self.voice_state.get("session_id")
        token = self.voice_state.get("token")
        endpoint = self.voice_state.get("endpoint")

        if not (session_id and token and endpoint):
            print(
                f"[Player {self.guild_id}] Handshake not finished. Queued play for when connected."
            )
            self.play_on_connect = True
            return

        try:
            self.playing = True
            await self._rust_player.set_volume(self.volume / 100.0)

            def event_callback(err, event_json):
                if err:
                    self.emit("error", self, err)
                    return
                try:
                    event = json.loads(event_json)
                    event_type = event.get("type")
                    if event_type == "trackStart":
                        self.emit("track_start", self, current_track)
                    elif event_type == "trackEnd":
                        self.playing = False
                        self.emit(
                            "track_end",
                            self,
                            current_track,
                            event.get("reason", "FINISHED"),
                        )
                        asyncio.create_task(self._handle_track_end())
                    elif event_type == "position":
                        self.emit("position", self, event.get("position"))
                except Exception as parse_err:
                    self.emit("error", self, parse_err)

            await self._rust_player.play(
                self.manager.client["id"],
                self.voice_channel_id,
                session_id,
                token,
                endpoint,
                current_track.info.uri,
                event_callback,
            )
            if self.paused:
                await self._rust_player.pause()
        except Exception as err:
            self.playing = False
            self.emit("error", self, err)
            raise

    async def _handle_track_end(self) -> None:
        finished_track = self.queue.current
        if finished_track:
            if self.repeat_mode == "track":
                await self.play()
            elif self.repeat_mode == "queue":
                self.queue.add(finished_track)
                self.queue.current = None
                await self.play()
            else:
                self.queue.previous.append(finished_track)
                self.queue.current = None
                await self.play()
        else:
            await self.play()

    async def pause(self, pause_state: bool = True) -> None:
        self.paused = pause_state
        if pause_state:
            await self._rust_player.pause()
        else:
            await self._rust_player.resume()

    async def resume(self) -> None:
        await self.pause(False)

    async def stop(self) -> None:
        self.playing = False
        self.queue.current = None
        await self._rust_player.stop()

    async def skip(self) -> None:
        await self._rust_player.stop()

    async def seek(self, position_ms: int) -> None:
        await self._rust_player.seek(position_ms)

    async def set_volume(self, volume: int) -> None:
        self.volume = volume
        await self._rust_player.set_volume(volume / 100.0)

    def set_repeat_mode(self, mode: Literal["off", "track", "queue"]) -> None:
        self.repeat_mode = mode

    async def set_filters(self, filters: Dict[str, Any]) -> None:
        await self._rust_player.set_filters(json.dumps(filters))

    def get_position(self) -> int:
        return self._rust_player.get_position()

    def is_paused(self) -> bool:
        return self._rust_player.is_paused()

    def __repr__(self) -> str:
        return f"Player(guild_id='{self.guild_id}', playing={self.playing})"


class LavendeManager:
    def __init__(self, send_to_shard: Callable, client: Dict[str, str]):
        self.players: Dict[str, Player] = {}
        self.send_to_shard = send_to_shard
        self.client = client
        self.node_manager: Dict[str, Any] = {"nodes": {}}
        self._event_handlers: Dict[str, list] = {}

    def on(self, event: str, handler: Callable) -> None:
        self._event_handlers.setdefault(event, []).append(handler)

    def emit(self, event: str, *args, **kwargs) -> None:
        for handler in self._event_handlers.get(event, []):
            if asyncio.iscoroutinefunction(handler):
                asyncio.create_task(handler(*args, **kwargs))
            else:
                handler(*args, **kwargs)

    def init(self, client_data: Optional[Dict[str, str]] = None) -> None:
        if client_data:
            self.client = client_data

    def create_player(self, options: Dict[str, Any]) -> Player:
        guild_id = options["guild_id"]
        if guild_id in self.players:
            return self.players[guild_id]
        player = Player(self, options)
        self.players[guild_id] = player
        self.emit("player_create", player)
        player.on("track_start", lambda p, t: self.emit("track_start", p, t))
        player.on("track_end", lambda p, t, r: self.emit("track_end", p, t, r))
        player.on("queue_end", lambda p: self.emit("queue_end", p))
        player.on("player_destroy", lambda p, r: self.emit("player_destroy", p, r))
        player.on("error", lambda p, err: self.emit("error", p, err))
        return player

    async def destroy_player(self, guild_id: str) -> None:
        player = self.players.get(guild_id)
        if player:
            await player.destroy()

    async def send_raw_data(self, packet: Dict[str, Any]) -> None:
        if not packet or "t" not in packet:
            return
        event_type = packet["t"]
        if event_type == "VOICE_STATE_UPDATE":
            data = packet["d"]
            if data.get("user_id") == self.client.get("id"):
                player = self.players.get(data.get("guild_id"))
                if player:
                    player.set_voice_state({"session_id": data.get("session_id")})
                    player.voice_channel_id = data.get("channel_id")
                    player.check_play_on_connect()
        elif event_type == "VOICE_SERVER_UPDATE":
            data = packet["d"]
            player = self.players.get(data.get("guild_id"))
            if player:
                player.set_voice_state(
                    {"token": data.get("token"), "endpoint": data.get("endpoint")}
                )
                player.check_play_on_connect()

    def __repr__(self) -> str:
        return f"LavendeManager(players={len(self.players)}, client_id='{self.client.get('id')}')"


LavendePlayer = Player


async def load(identifier: str, requester: Any = None) -> Dict[str, Any]:
    json_str = await _rust_load(identifier)
    data = json.loads(json_str)
    result: Dict[str, Any] = {"loadType": "empty", "tracks": []}
    load_type = data.get("loadType", "empty")
    if load_type == "track":
        result["loadType"] = "track"
        result["tracks"] = [Track(data["data"], requester)]
    elif load_type == "playlist":
        result["loadType"] = "playlist"
        result["playlistInfo"] = data["data"]["info"]
        result["tracks"] = [Track(t, requester) for t in data["data"]["tracks"]]
    elif load_type == "search":
        result["loadType"] = "search"
        result["tracks"] = [Track(t, requester) for t in data["data"]]
    elif load_type == "error":
        result["loadType"] = "error"
        result["exception"] = data["data"]
    return result


__all__ = [
    "Track",
    "TrackInfo",
    "Queue",
    "QueueUtils",
    "FilterManager",
    "Player",
    "LavendePlayer",
    "LavendeManager",
    "load",
    "_RustPlayer",
    "DebugEvents",
    "DestroyReasons",
    "DisconnectReasons",
    "VALID_SPONSOR_BLOCKS",
    "AUDIO_OUTPUTS_DATA",
    "EQ_LIST",
    "DEFAULT_SOURCES",
    "SOURCE_LINKS_REGEXES",
]
