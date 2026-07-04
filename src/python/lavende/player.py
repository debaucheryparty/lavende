from typing import Optional, Dict, Any, Callable, Literal, Union
import asyncio
import json
from lavende._lavende import Player as _RustPlayer
from lavende.queue import Queue
from lavende.filter_manager import FilterManager
from lavende.track import Track


class Player:
    def __init__(self, manager, options: Dict[str, Any]):
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
        if event not in self._event_handlers:
            self._event_handlers[event] = []
        self._event_handlers[event].append(handler)
    
    def emit(self, event: str, *args, **kwargs) -> None:
        if event in self._event_handlers:
            for handler in self._event_handlers[event]:
                if asyncio.iscoroutinefunction(handler):
                    asyncio.create_task(handler(*args, **kwargs))
                else:
                    handler(*args, **kwargs)
    
    def set(self, key: str, value: Any) -> 'Player':
        self._data[key] = value
        return self
    
    def get(self, key: str, default: Any = None) -> Any:
        return self._data.get(key, default)
    
    def set_data(self, key: str, value: Any) -> 'Player':
        return self.set(key, value)
    
    def get_data(self, key: str, default: Any = None) -> Any:
        return self.get(key, default)
    
    def delete_data(self, key: str) -> 'Player':
        self._data.pop(key, None)
        return self
    
    def clear_data(self) -> 'Player':
        self._data = {}
        return self
    
    def get_all_data(self) -> Dict[str, Any]:
        return self._data.copy()
    
    def set_voice_state(self, state: Dict[str, Optional[str]]) -> None:
        self.voice_state.update(state)
    
    def check_play_on_connect(self) -> None:
        session_id = self.voice_state.get("session_id")
        token = self.voice_state.get("token")
        endpoint = self.voice_state.get("endpoint")
        
        if session_id and token and endpoint and self.play_on_connect:
            self.play_on_connect = False
            print(f"[Player {self.guild_id}] Delayed play handshake completed, starting playback.")
            asyncio.create_task(self._safe_play())
    
    async def _safe_play(self):
        try:
            await self.play()
        except Exception as e:
            self.emit("error", self, e)
    
    async def connect(self) -> None:
        await self.manager.send_to_shard(self.guild_id, {
            "op": 4,
            "d": {
                "guild_id": self.guild_id,
                "channel_id": self.voice_channel_id,
                "self_mute": False,
                "self_deaf": self.self_deaf,
            },
        })
    
    async def disconnect(self) -> None:
        self.voice_channel_id = None
        await self.manager.send_to_shard(self.guild_id, {
            "op": 4,
            "d": {
                "guild_id": self.guild_id,
                "channel_id": None,
                "self_mute": False,
                "self_deaf": False,
            },
        })
        await self.stop()
    
    async def destroy(self, reason: Optional[str] = None) -> None:
        await self.disconnect()
        self.emit("player_destroy", self, reason)
        self.manager.players.pop(self.guild_id, None)
    
    async def search(
        self, 
        query: Union[str, Dict[str, str]], 
        requester: Any = None
    ) -> Dict[str, Any]:
        from lavende.utils import load
        
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
            print(f"[Player {self.guild_id}] Handshake not finished. Queued play for when connected.")
            self.play_on_connect = True
            return
        
        try:
            self.playing = True
            float_volume = self.volume / 100.0
            await self._rust_player.set_volume(float_volume)
            
            print(f"[Player {self.guild_id}] Invoking native Player.play with params:",
                  {"client_id": self.manager.client["id"], "voice_channel_id": self.voice_channel_id,
                   "session_id": session_id, "token": token, "endpoint": endpoint,
                   "identifier": current_track.info.uri})
            
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
                        self.emit("track_end", self, current_track, event.get("reason", "FINISHED"))
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
                event_callback
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
        float_volume = volume / 100.0
        await self._rust_player.set_volume(float_volume)
    
    def set_repeat_mode(self, mode: Literal["off", "track", "queue"]) -> None:
        self.repeat_mode = mode
    
    async def set_filters(self, filters: Dict[str, Any]) -> None:
        json_str = json.dumps(filters)
        await self._rust_player.set_filters(json_str)
    
    def get_position(self) -> int:
        return self._rust_player.get_position()
    
    def is_paused(self) -> bool:
        return self._rust_player.is_paused()
    
    def __repr__(self) -> str:
        return f"Player(guild_id='{self.guild_id}', playing={self.playing})"
