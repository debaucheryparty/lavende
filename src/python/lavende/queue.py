from typing import List, Optional, Callable, Union
import random
from lavende.track import Track


class QueueUtils:
    def __init__(self, queue: 'Queue'):
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
    
    def add(self, track: Union[Track, List[Track]], index: Optional[int] = None) -> None:
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
