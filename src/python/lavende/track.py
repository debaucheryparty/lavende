from typing import Optional


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
