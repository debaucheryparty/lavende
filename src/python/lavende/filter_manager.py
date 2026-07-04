from typing import TYPE_CHECKING, List, Dict, Any, Literal

if TYPE_CHECKING:
    from lavende.player import Player


class FilterManager:
    def __init__(self, player: 'Player'):
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
    
    async def reset_filters(self) -> 'FilterManager':
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
    
    async def set_volume(self, volume: float) -> 'FilterManager':
        self.data["volume"] = volume
        self.filters["volume"] = volume != 1.0
        await self.apply_player_filters()
        return self
    
    async def set_audio_output(
        self, 
        output_type: Literal["mono", "stereo", "left", "right"]
    ) -> 'FilterManager':
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
    
    async def set_speed(self, speed: float = 1.0) -> 'FilterManager':
        if "timescale" not in self.data:
            self.data["timescale"] = {}
        self.data["timescale"]["speed"] = speed
        await self.apply_player_filters()
        return self
    
    async def set_pitch(self, pitch: float = 1.0) -> 'FilterManager':
        if "timescale" not in self.data:
            self.data["timescale"] = {}
        self.data["timescale"]["pitch"] = pitch
        await self.apply_player_filters()
        return self
    
    async def set_rate(self, rate: float = 1.0) -> 'FilterManager':
        if "timescale" not in self.data:
            self.data["timescale"] = {}
        self.data["timescale"]["rate"] = rate
        await self.apply_player_filters()
        return self
    
    async def toggle_rotation(self, rotation_hz: float = 0.2) -> 'FilterManager':
        if self.filters["rotation"]:
            if "rotation" in self.data:
                del self.data["rotation"]
        else:
            self.data["rotation"] = {"rotationHz": rotation_hz}
        
        self.filters["rotation"] = not self.filters["rotation"]
        await self.apply_player_filters()
        return self
    
    async def toggle_vibrato(
        self, 
        frequency: float = 10.0, 
        depth: float = 1.0
    ) -> 'FilterManager':
        if self.filters["vibrato"]:
            if "vibrato" in self.data:
                del self.data["vibrato"]
        else:
            self.data["vibrato"] = {"frequency": frequency, "depth": depth}
        
        self.filters["vibrato"] = not self.filters["vibrato"]
        await self.apply_player_filters()
        return self
    
    async def toggle_tremolo(
        self, 
        frequency: float = 4.0, 
        depth: float = 0.8
    ) -> 'FilterManager':
        if self.filters["tremolo"]:
            if "tremolo" in self.data:
                del self.data["tremolo"]
        else:
            self.data["tremolo"] = {"frequency": frequency, "depth": depth}
        
        self.filters["tremolo"] = not self.filters["tremolo"]
        await self.apply_player_filters()
        return self
