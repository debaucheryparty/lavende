from enum import Enum


class DebugEvents(str, Enum):
    SET_SPONSOR_BLOCK = "SetSponsorBlock"
    DELETE_SPONSOR_BLOCK = "DeleteSponsorBlock"
    TRACK_END_REPLACED = "TrackEndReplaced"
    AUTOPLAY_EXECUTION = "AutoplayExecution"
    AUTOPLAY_NO_SONGS_ADDED = "AutoplayNoSongsAdded"
    AUTOPLAY_THRESHOLD_SPAM_LIMITER = "AutoplayThresholdSpamLimiter"
    TRIGGER_QUEUE_EMPTY_INTERVAL = "TriggerQueueEmptyInterval"
    QUEUE_ENDED = "QueueEnded"
    TRACK_START_NEW_SONGS_ONLY = "TrackStartNewSongsOnly"
    TRACK_START_NO_TRACK = "TrackStartNoTrack"
    RESUMING_FETCHING_ERROR = "ResumingFetchingError"
    PLAYER_UPDATE_NO_PLAYER = "PlayerUpdateNoPlayer"
    PLAYER_UPDATE_FILTER_FIX_APPLY = "PlayerUpdateFilterFixApply"
    PLAYER_UPDATE_SUCCESS = "PlayerUpdateSuccess"
    HEART_BEAT_TRIGGERED = "HeartBeatTriggered"
    NO_SOCKET_ON_DESTROY = "NoSocketOnDestroy"
    SOCKET_CLEANUP_ERROR = "SocketCleanupError"
    SOCKET_TERMINATE_HEART_BEAT_TIMEOUT = "SocketTerminateHeartBeatTimeout"
    TRYING_CONNECT_WHILE_CONNECTED = "TryingConnectWhileConnected"
    LAVA_SEARCH_NOTHING_FOUND = "LavaSearchNothingFound"
    SEARCH_NOTHING_FOUND = "SearchNothingFound"
    VALIDATING_BLACKLIST_LINKS = "ValidatingBlacklistLinks"
    VALIDATING_WHITELIST_LINKS = "ValidatingWhitelistLinks"
    TRACK_ERROR_MAX_TRACKS_ERRORED_PER_TIME = "TrackErrorMaxTracksErroredPerTime"
    TRACK_STUCK_MAX_TRACKS_ERRORED_PER_TIME = "TrackStuckMaxTracksErroredPerTime"
    PLAYER_DESTROYING_SOMEWHERE_ELSE = "PlayerDestroyingSomewhereElse"
    PLAYER_CREATE_NODE_NOT_FOUND = "PlayerCreateNodeNotFound"
    PLAYER_PLAY_QUEUE_EMPTY_TIMEOUT_CLEAR = "PlayerPlayQueueEmptyTimeoutClear"
    PLAYER_PLAY_WITH_TRACK_REPLACE = "PlayerPlayWithTrackReplace"
    PLAYER_PLAY_UNRESOLVED_TRACK = "PlayerPlayUnresolvedTrack"
    PLAYER_PLAY_UNRESOLVED_TRACK_FAILED = "PlayerPlayUnresolvedTrackFailed"
    PLAYER_VOLUME_AS_FILTER = "PlayerVolumeAsFilter"
    BANDCAMP_SEARCH_LOKAL_ENGINE = "BandcampSearchLokalEngine"
    PLAYER_CHANGE_NODE = "PlayerChangeNode"
    BUILD_TRACK_ERROR = "BuildTrackError"
    TRANSFORM_REQUESTER_FUNCTION_FAILED = "TransformRequesterFunctionFailed"
    GET_CLOSEST_TRACK_FAILED = "GetClosestTrackFailed"
    PLAYER_DELETE_INSTEAD_OF_DESTROY = "PlayerDeleteInsteadOfDestroy"
    FAILED_TO_CONNECT_TO_NODES = "FailedToConnectToNodes"
    NO_AUDIO_DEBUG = "NoAudioDebug"
    PLAYER_AUTO_RECONNECT = "PlayerAutoReconnect"
    PLAYER_DESTROY_FAIL = "PlayerDestroyFail"
    PLAYER_CHANGE_NODE_FAIL_NO_ELIGIBLE_NODE = "PlayerChangeNodeFailNoEligibleNode"
    PLAYER_CHANGE_NODE_FAIL = "PlayerChangeNodeFail"


class DestroyReasons(str, Enum):
    QUEUE_EMPTY = "QueueEmpty"
    NODE_DESTROY = "NodeDestroy"
    NODE_DELETED = "NodeDeleted"
    LAVALINK_NO_VOICE = "LavalinkNoVoice"
    NODE_RECONNECT_FAIL = "NodeReconnectFail"
    DISCONNECTED = "Disconnected"
    PLAYER_RECONNECT_FAIL = "PlayerReconnectFail"
    PLAYER_CHANGE_NODE_FAIL = "PlayerChangeNodeFail"
    PLAYER_CHANGE_NODE_FAIL_NO_ELIGIBLE_NODE = "PlayerChangeNodeFailNoEligibleNode"
    CHANNEL_DELETED = "ChannelDeleted"
    DISCONNECT_ALL_NODES = "DisconnectAllNodes"
    RECONNECT_ALL_NODES = "ReconnectAllNodes"
    TRACK_ERROR_MAX_TRACKS_ERRORED_PER_TIME = "TrackErrorMaxTracksErroredPerTime"
    TRACK_STUCK_MAX_TRACKS_ERRORED_PER_TIME = "TrackStuckMaxTracksErroredPerTime"


class DisconnectReasons(str, Enum):
    DISCONNECTED = "Disconnected"
    DISCONNECT_ALL_NODES = "DisconnectAllNodes"


VALID_SPONSOR_BLOCKS = [
    "sponsor",
    "selfpromo",
    "interaction",
    "intro",
    "outro",
    "preview",
    "music_offtopic",
    "filler",
]

AUDIO_OUTPUTS_DATA = {
    "mono": {
        "leftToLeft": 0.5,
        "leftToRight": 0.5,
        "rightToLeft": 0.5,
        "rightToRight": 0.5,
    },
    "stereo": {
        "leftToLeft": 1,
        "leftToRight": 0,
        "rightToLeft": 0,
        "rightToRight": 1,
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
}

EQ_LIST = {
    "BassboostEarrape": [
        {"band": 0, "gain": 0.225},
        {"band": 1, "gain": 0.25125},
        {"band": 2, "gain": 0.25125},
        {"band": 3, "gain": 0.15},
        {"band": 4, "gain": -0.1875},
        {"band": 5, "gain": 0.05625},
        {"band": 6, "gain": -0.16875},
        {"band": 7, "gain": 0.08625},
        {"band": 8, "gain": 0.13125},
        {"band": 9, "gain": 0.16875},
    ],
}

DEFAULT_SOURCES = {
    "youtube music": "ytmsearch",
    "youtubemusic": "ytmsearch",
    "ytmsearch": "ytmsearch",
    "ytm": "ytmsearch",
    "musicyoutube": "ytmsearch",
    "music youtube": "ytmsearch",
    "youtube": "ytsearch",
    "yt": "ytsearch",
    "ytsearch": "ytsearch",
    "soundcloud": "scsearch",
    "scsearch": "scsearch",
    "sc": "scsearch",
    "apple music": "amsearch",
    "apple": "amsearch",
    "applemusic": "amsearch",
    "amsearch": "amsearch",
    "am": "amsearch",
    "musicapple": "amsearch",
    "music apple": "amsearch",
    "spotify": "spsearch",
    "spsearch": "spsearch",
    "sp": "spsearch",
    "spotify.com": "spsearch",
    "spotifycom": "spsearch",
    "sprec": "sprec",
    "spsuggestion": "sprec",
    "deezer": "dzsearch",
    "dz": "dzsearch",
    "dzsearch": "dzsearch",
    "dzisrc": "dzisrc",
    "dzrec": "dzrec",
    "yandex music": "ymsearch",
    "yandexmusic": "ymsearch",
    "yandex": "ymsearch",
    "ymsearch": "ymsearch",
    "ymrec": "ymrec",
    "vksearch": "vksearch",
    "vkmusic": "vksearch",
    "vk music": "vksearch",
    "vkrec": "vkrec",
    "vk": "vksearch",
    "qbsearch": "qbsearch",
    "qobuz": "qbsearch",
    "qbisrc": "qbisrc",
    "qbrec": "qbrec",
    "pandora": "pdsearch",
    "pd": "pdsearch",
    "pdsearch": "pdsearch",
    "pandora music": "pdsearch",
    "pandoramusic": "pdsearch",
    "speak": "speak",
    "tts": "tts",
    "ftts": "ftts",
    "flowery": "ftts",
    "flowery.tts": "ftts",
    "flowerytts": "ftts",
    "bandcamp": "bcsearch",
    "bc": "bcsearch",
    "bcsearch": "bcsearch",
    "phsearch": "phsearch",
    "pornhub": "phsearch",
    "porn": "phsearch",
    "local": "local",
    "http": "http",
    "https": "https",
    "link": "link",
    "uri": "uri",
    "tidal": "tdsearch",
    "td": "tdsearch",
    "tidal music": "tdsearch",
    "tdrec": "tdrec",
    "jiosaavn": "jssearch",
    "js": "jssearch",
    "jssearch": "jssearch",
    "jsrec": "jsrec",
    "amzsearch": "amzsearch",
    "admsearch": "admsearch",
    "gnsearch": "gnsearch",
    "szsearch": "szsearch",
}

SOURCE_LINKS_REGEXES = {
    "YoutubeRegex": r'https?:\/\/?(?:www\.)?(?:(m|www)\.)?(?:youtu\.be\/|youtube\.com\/(?:embed\/|v\/|shorts|playlist\?|watch\?v=|watch\?.+(?:&|&#38;);v=))([a-zA-Z0-9\-_]{11})?(?:(?:\?|&|&#38;)index=((?:\d){1,3}))?(?:(?:\?|&|&#38;)?list=([a-zA-Z\-_0-9]{34}))?(?:\S+)?',
    "YoutubeMusicRegex": r'https?:\/\/?(?:www\.)?(?:(music|m|www)\.)?(?:youtu\.be\/|youtube\.com\/(?:embed\/|v\/|shorts|playlist\?|watch\?v=|watch\?.+(?:&|&#38;);v=))([a-zA-Z0-9\-_]{11})?(?:(?:\?|&|&#38;)index=((?:\d){1,3}))?(?:(?:\?|&|&#38;)?list=([a-zA-Z\-_0-9]{34}))?(?:\S+)?',
    "SoundCloudRegex": r'https?:\/\/(?:on\.)?soundcloud\.com\/',
    "SoundCloudMobileRegex": r'https?:\/\/(soundcloud\.app\.goo\.gl)\/(\S+)',
    "bandcamp": r'https?:\/\/?(?:www\.)?([\d|\w]+)\.bandcamp\.com\/(\S+)',
    "TwitchTv": r'https?:\/\/?(?:www\.)?twitch\.tv\/\w+',
    "vimeo": r'https?:\/\/(www\.)?vimeo.com\/(?:channels\/(?:\w+\/)?|groups\/([^/]*)\/videos\/|)(\d+)(?:|\/\?)',
    "mp3Url": r'(https?|ftp|file):\/\/(www.)?(.*?)\.(mp3)$',
    "m3uUrl": r'(https?|ftp|file):\/\/(www.)?(.*?)\.(m3u)$',
    "m3u8Url": r'(https?|ftp|file):\/\/(www.)?(.*?)\.(m3u8)$',
    "mp4Url": r'(https?|ftp|file):\/\/(www.)?(.*?)\.(mp4)$',
    "m4aUrl": r'(https?|ftp|file):\/\/(www.)?(.*?)\.(m4a)$',
    "wavUrl": r'(https?|ftp|file):\/\/(www.)?(.*?)\.(wav)$',
    "aacpUrl": r'(https?|ftp|file):\/\/(www.)?(.*?)\.(aacp)$',
    "DeezerTrackRegex": r'(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?track\/(\d+)',
    "DeezerPageLinkRegex": r'(https?:\/\/|)?(?:www\.)?deezer\.page\.link\/(\S+)',
    "DeezerPlaylistRegex": r'(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?playlist\/(\d+)',
    "DeezerAlbumRegex": r'(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?album\/(\d+)',
    "DeezerArtistRegex": r'(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?artist\/(\d+)',
    "DeezerMixesRegex": r'(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?mixes\/genre\/(\d+)',
    "DeezerEpisodeRegex": r'(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?episode\/(\d+)',
    "AllDeezerRegexWithoutPageLink": r'(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?(track|playlist|album|artist|mixes\/genre|episode)\/(\d+)',
    "AllDeezerRegex": r'((https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?(track|playlist|album|artist|mixes\/genre|episode)\/(\d+)|(https?:\/\/|)?(?:www\.)?deezer\.page\.link\/(\S+))',
    "SpotifySongRegex": r'(https?:\/\/)(www\.)?open\.spotify\.com\/((?P<region>[a-zA-Z-]+)\/)?(user\/(?P<user>[a-zA-Z0-9-_]+)\/)?track\/(?P<identifier>[a-zA-Z0-9-_]+)',
    "SpotifyPlaylistRegex": r'(https?:\/\/)(www\.)?open\.spotify\.com\/((?P<region>[a-zA-Z-]+)\/)?(user\/(?P<user>[a-zA-Z0-9-_]+)\/)?playlist\/(?P<identifier>[a-zA-Z0-9-_]+)',
    "SpotifyArtistRegex": r'(https?:\/\/)(www\.)?open\.spotify\.com\/((?P<region>[a-zA-Z-]+)\/)?(user\/(?P<user>[a-zA-Z0-9-_]+)\/)?artist\/(?P<identifier>[a-zA-Z0-9-_]+)',
    "SpotifyEpisodeRegex": r'(https?:\/\/)(www\.)?open\.spotify\.com\/((?P<region>[a-zA-Z-]+)\/)?(user\/(?P<user>[a-zA-Z0-9-_]+)\/)?episode\/(?P<identifier>[a-zA-Z0-9-_]+)',
    "SpotifyShowRegex": r'(https?:\/\/)(www\.)?open\.spotify\.com\/((?P<region>[a-zA-Z-]+)\/)?(user\/(?P<user>[a-zA-Z0-9-_]+)\/)?show\/(?P<identifier>[a-zA-Z0-9-_]+)',
    "SpotifyAlbumRegex": r'(https?:\/\/)(www\.)?open\.spotify\.com\/((?P<region>[a-zA-Z-]+)\/)?(user\/(?P<user>[a-zA-Z0-9-_]+)\/)?album\/(?P<identifier>[a-zA-Z0-9-_]+)',
    "AllSpotifyRegex": r'(https?:\/\/)(www\.)?open\.spotify\.com\/((?P<region>[a-zA-Z-]+)\/)?(user\/(?P<user>[a-zA-Z0-9-_]+)\/)?(?P<type>track|album|playlist|artist|episode|show)\/(?P<identifier>[a-zA-Z0-9-_]+)',
    "appleMusic": r'https?:\/\/?(?:www\.)?music\.apple\.com\/(\S+)',
    "tidal": r'https?:\/\/?(?:www\.)?(?:tidal|listen)\.tidal\.com\/(?P<type>track|album|playlist|artist)\/(?P<identifier>[a-zA-Z0-9-_]+)',
    "jiosaavn": r'(https?:\/\/)(www\.)?jiosaavn\.com\/(?P<type>song|album|featured|artist)\/([a-zA-Z0-9-_/,]+)',
    "PandoraTrackRegex": r'^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/artist\/[\w-]+(?:\/[\w-]+)*\/(?P<identifier>TR[A-Za-z0-9]+)(?:[?#].*)?$',
    "PandoraAlbumRegex": r'^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/artist\/[\w-]+(?:\/[\w-]+)*\/(?P<identifier>AL[A-Za-z0-9]+)(?:[?#].*)?$',
    "PandoraArtistRegex": r'^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/artist\/[\w-]+\/(?P<identifier>AR[A-Za-z0-9]+)(?:[?#].*)?$',
    "PandoraPlaylistRegex": r'^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/playlist\/(?P<identifier>PL:[\d:]+)(?:[?#].*)?$',
    "AllPandoraRegex": r'^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/(?:playlist\/(?P<playlistId>PL:[\d:]+)|artist\/[\w-]+(?:\/[\w-]+)*\/(?P<identifier>(?:TR|AL|AR)[A-Za-z0-9]+))(?:[?#].*)?$',
    "tiktok": r'https:\/\/www\.tiktok\.com\/',
    "mixcloud": r'https:\/\/www\.mixcloud\.com\/',
    "musicYandex": r'https:\/\/music\.yandex\.ru\/',
    "radiohost": r'https?:\/\/[^.\s]+\.radiohost\.de\/(\S+)',
}
