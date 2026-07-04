from typing import Dict, Callable, Any, Optional
import asyncio
from lavende.player import Player


class LavendeManager:
    def __init__(self, send_to_shard: Callable, client: Dict[str, str]):
        self.players: Dict[str, Player] = {}
        self.send_to_shard = send_to_shard
        self.client = client
        self.node_manager = {
            "nodes": {},
        }
        self._event_handlers: Dict[str, list] = {}
    
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
                    player.set_voice_state({
                        "session_id": data.get("session_id"),
                    })
                    player.voice_channel_id = data.get("channel_id")
                    player.check_play_on_connect()
        
        elif event_type == "VOICE_SERVER_UPDATE":
            data = packet["d"]
            player = self.players.get(data.get("guild_id"))
            if player:
                player.set_voice_state({
                    "token": data.get("token"),
                    "endpoint": data.get("endpoint"),
                })
                player.check_play_on_connect()
    
    def __repr__(self) -> str:
        return f"LavendeManager(players={len(self.players)}, client_id='{self.client.get('id')}')"


LavendePlayer = Player
