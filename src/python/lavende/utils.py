import json
from typing import Any, Dict, List, Optional
from lavende._lavende import load as _rust_load
from lavende.track import Track


async def load(
    identifier: str,
    requester: Any = None
) -> Dict[str, Any]:

    json_str = await _rust_load(identifier)
    data = json.loads(json_str)
    
    result: Dict[str, Any] = {
        "loadType": "empty",
        "tracks": [],
    }
    
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
