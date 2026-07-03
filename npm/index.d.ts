import { EventEmitter } from 'events';
export interface TrackInfo {
    title: string;
    author: string;
    length: number;
    identifier: string;
    isStream: boolean;
    isSeekable: boolean;
    uri: string;
    sourceName: string;
    artworkUrl?: string;
    issrc?: string;
}
export declare class Track {
    encoded: string;
    info: TrackInfo;
    requester: any;
    constructor(data: {
        encoded: string;
        info: TrackInfo;
    }, requester: any);
}
export declare class Queue {
    tracks: Track[];
    current: Track | null;
    previous: Track[];
    guildId: string;
    constructor(guildId: string);
    get size(): number;
    add(track: Track | Track[], index?: number): void;
    remove(index: number): void;
    clear(): void;
    shuffle(): void;
    utils: {
        toJSON: () => {
            current: {
                encoded: string;
                info: TrackInfo;
                requester: any;
            } | null;
            previous: Track[];
            tracks: Track[];
        };
        totalDuration: () => number;
        filterTracks: (predicate: (track: Track, index: number) => boolean) => {
            track: Track;
            index: number;
        }[];
        findTrack: (predicate: (track: Track, index: number) => boolean) => {
            track: Track;
            index: number;
        } | null;
    };
}
export declare class FilterManager {
    player: Player;
    data: any;
    equalizerBands: {
        band: number;
        gain: number;
    }[];
    filters: any;
    constructor(player: Player);
    applyPlayerFilters(): Promise<void>;
    resetFilters(): Promise<FilterManager>;
    setVolume(volume: number): Promise<FilterManager>;
    setAudioOutput(type: 'mono' | 'stereo' | 'left' | 'right'): Promise<FilterManager>;
    setSpeed(speed?: number): Promise<FilterManager>;
    setPitch(pitch?: number): Promise<FilterManager>;
    setRate(rate?: number): Promise<FilterManager>;
    toggleRotation(rotationHz?: number): Promise<FilterManager>;
    toggleVibrato(frequency?: number, depth?: number): Promise<FilterManager>;
    toggleTremolo(frequency?: number, depth?: number): Promise<FilterManager>;
}
export interface PlayerOptions {
    guildId: string;
    voiceChannelId: string;
    textChannelId?: string;
    volume?: number;
    selfDeaf?: boolean;
}
export declare class Player extends EventEmitter {
    manager: LavendeManager;
    filterManager: FilterManager;
    private player;
    queue: Queue;
    guildId: string;
    voiceChannelId: string | null;
    textChannelId: string | null;
    volume: number;
    paused: boolean;
    playing: boolean;
    repeatMode: 'off' | 'track' | 'queue';
    selfDeaf: boolean;
    voiceState: {
        sessionId?: string;
        token?: string;
        endpoint?: string;
    };
    node: any;
    playOnConnect: boolean;
    private data;
    constructor(manager: LavendeManager, options: PlayerOptions);
    set(key: string, value: unknown): this;
    get<T>(key: string): T;
    setData(key: string, value: unknown): this;
    getData<T>(key: string): T;
    deleteData(key: string): this;
    clearData(): this;
    getAllData(): Record<string, unknown>;
    setVoiceState(state: {
        sessionId?: string;
        token?: string;
        endpoint?: string;
    }): void;
    checkPlayOnConnect(): void;
    connect(): Promise<void>;
    disconnect(): Promise<void>;
    destroy(reason?: string): Promise<void>;
    search(query: string | {
        query: string;
        source?: string;
    }, requester?: any): Promise<{
        loadType: 'track' | 'playlist' | 'search' | 'empty' | 'error';
        tracks: Track[];
        playlistInfo?: {
            name: string;
            selectedTrack?: number;
        };
        exception?: {
            message: string;
            severity: string;
        };
    }>;
    play(options?: {
        track?: Track;
        volume?: number;
        paused?: boolean;
    }): Promise<void>;
    private handleTrackEnd;
    pause(pauseState?: boolean): Promise<void>;
    resume(): Promise<void>;
    stop(): Promise<void>;
    skip(): Promise<void>;
    seek(positionMs: number): Promise<void>;
    setVolume(volume: number): Promise<void>;
    setRepeatMode(mode: 'off' | 'track' | 'queue'): void;
    setFilters(filters: any): Promise<void>;
    getPosition(): number;
    isPaused(): boolean;
}
export interface LavendeManagerOptions {
    sendToShard: (guildId: string, payload: any) => void;
    client: {
        id: string;
        username?: string;
    };
}
export declare class LavendeManager extends EventEmitter {
    players: Map<string, Player>;
    sendToShard: (guildId: string, payload: any) => void;
    client: {
        id: string;
        username?: string;
    };
    nodeManager: {
        nodes: Map<any, any>;
    };
    constructor(options: LavendeManagerOptions);
    init(clientData?: {
        id: string;
        username?: string;
    }): void;
    createPlayer(options: PlayerOptions): Player;
    destroyPlayer(guildId: string): void;
    sendRawData(packet: any): void;
}
export { Player as LavendePlayer };
export declare function load(identifier: string, requester?: any): Promise<{
    loadType: 'track' | 'playlist' | 'search' | 'empty' | 'error';
    tracks: Track[];
    playlistInfo?: {
        name: string;
        selectedTrack?: number;
    };
    exception?: {
        message: string;
        severity: string;
    };
}>;
export declare class MiniMap<K, V> extends Map<K, V> {
    constructor(data?: [K, V][]);
    filter(fn: (value: V, key: K, miniMap: this) => boolean): MiniMap<K, V>;
    map<T>(fn: (value: V, key: K, miniMap: this) => T): T[];
    toJSON(): [K, V][];
}
export declare enum DebugEvents {
    SetSponsorBlock = "SetSponsorBlock",
    DeleteSponsorBlock = "DeleteSponsorBlock",
    TrackEndReplaced = "TrackEndReplaced",
    AutoplayExecution = "AutoplayExecution",
    AutoplayNoSongsAdded = "AutoplayNoSongsAdded",
    AutoplayThresholdSpamLimiter = "AutoplayThresholdSpamLimiter",
    TriggerQueueEmptyInterval = "TriggerQueueEmptyInterval",
    QueueEnded = "QueueEnded",
    TrackStartNewSongsOnly = "TrackStartNewSongsOnly",
    TrackStartNoTrack = "TrackStartNoTrack",
    ResumingFetchingError = "ResumingFetchingError",
    PlayerUpdateNoPlayer = "PlayerUpdateNoPlayer",
    PlayerUpdateFilterFixApply = "PlayerUpdateFilterFixApply",
    PlayerUpdateSuccess = "PlayerUpdateSuccess",
    HeartBeatTriggered = "HeartBeatTriggered",
    NoSocketOnDestroy = "NoSocketOnDestroy",
    SocketCleanupError = "SocketCleanupError",
    SocketTerminateHeartBeatTimeout = "SocketTerminateHeartBeatTimeout",
    TryingConnectWhileConnected = "TryingConnectWhileConnected",
    LavaSearchNothingFound = "LavaSearchNothingFound",
    SearchNothingFound = "SearchNothingFound",
    ValidatingBlacklistLinks = "ValidatingBlacklistLinks",
    ValidatingWhitelistLinks = "ValidatingWhitelistLinks",
    TrackErrorMaxTracksErroredPerTime = "TrackErrorMaxTracksErroredPerTime",
    TrackStuckMaxTracksErroredPerTime = "TrackStuckMaxTracksErroredPerTime",
    PlayerDestroyingSomewhereElse = "PlayerDestroyingSomewhereElse",
    PlayerCreateNodeNotFound = "PlayerCreateNodeNotFound",
    PlayerPlayQueueEmptyTimeoutClear = "PlayerPlayQueueEmptyTimeoutClear",
    PlayerPlayWithTrackReplace = "PlayerPlayWithTrackReplace",
    PlayerPlayUnresolvedTrack = "PlayerPlayUnresolvedTrack",
    PlayerPlayUnresolvedTrackFailed = "PlayerPlayUnresolvedTrackFailed",
    PlayerVolumeAsFilter = "PlayerVolumeAsFilter",
    BandcampSearchLokalEngine = "BandcampSearchLokalEngine",
    PlayerChangeNode = "PlayerChangeNode",
    BuildTrackError = "BuildTrackError",
    TransformRequesterFunctionFailed = "TransformRequesterFunctionFailed",
    GetClosestTrackFailed = "GetClosestTrackFailed",
    PlayerDeleteInsteadOfDestroy = "PlayerDeleteInsteadOfDestroy",
    FailedToConnectToNodes = "FailedToConnectToNodes",
    NoAudioDebug = "NoAudioDebug",
    PlayerAutoReconnect = "PlayerAutoReconnect",
    PlayerDestroyFail = "PlayerDestroyFail",
    PlayerChangeNodeFailNoEligibleNode = "PlayerChangeNodeFailNoEligibleNode",
    PlayerChangeNodeFail = "PlayerChangeNodeFail"
}
export declare enum DestroyReasons {
    QueueEmpty = "QueueEmpty",
    NodeDestroy = "NodeDestroy",
    NodeDeleted = "NodeDeleted",
    LavalinkNoVoice = "LavalinkNoVoice",
    NodeReconnectFail = "NodeReconnectFail",
    Disconnected = "Disconnected",
    PlayerReconnectFail = "PlayerReconnectFail",
    PlayerChangeNodeFail = "PlayerChangeNodeFail",
    PlayerChangeNodeFailNoEligibleNode = "PlayerChangeNodeFailNoEligibleNode",
    ChannelDeleted = "ChannelDeleted",
    DisconnectAllNodes = "DisconnectAllNodes",
    ReconnectAllNodes = "ReconnectAllNodes",
    TrackErrorMaxTracksErroredPerTime = "TrackErrorMaxTracksErroredPerTime",
    TrackStuckMaxTracksErroredPerTime = "TrackStuckMaxTracksErroredPerTime"
}
export declare enum DisconnectReasons {
    Disconnected = "Disconnected",
    DisconnectAllNodes = "DisconnectAllNodes"
}
export declare const validSponsorBlocks: string[];
export declare const audioOutputsData: {
    mono: {
        leftToLeft: number;
        leftToRight: number;
        rightToLeft: number;
        rightToRight: number;
    };
    stereo: {
        leftToLeft: number;
        leftToRight: number;
        rightToLeft: number;
        rightToRight: number;
    };
    left: {
        leftToLeft: number;
        leftToRight: number;
        rightToLeft: number;
        rightToRight: number;
    };
    right: {
        leftToLeft: number;
        leftToRight: number;
        rightToLeft: number;
        rightToRight: number;
    };
};
export declare const EQList: {
    BassboostEarrape: {
        band: number;
        gain: number;
    }[];
};
export declare const DefaultSources: Record<string, string>;
export declare const SourceLinksRegexes: {
    YoutubeRegex: RegExp;
    YoutubeMusicRegex: RegExp;
    SoundCloudRegex: RegExp;
    SoundCloudMobileRegex: RegExp;
    bandcamp: RegExp;
    TwitchTv: RegExp;
    vimeo: RegExp;
    mp3Url: RegExp;
    m3uUrl: RegExp;
    m3u8Url: RegExp;
    mp4Url: RegExp;
    m4aUrl: RegExp;
    wavUrl: RegExp;
    aacpUrl: RegExp;
    DeezerTrackRegex: RegExp;
    DeezerPageLinkRegex: RegExp;
    DeezerPlaylistRegex: RegExp;
    DeezerAlbumRegex: RegExp;
    DeezerArtistRegex: RegExp;
    DeezerMixesRegex: RegExp;
    DeezerEpisodeRegex: RegExp;
    AllDeezerRegexWithoutPageLink: RegExp;
    AllDeezerRegex: RegExp;
    SpotifySongRegex: RegExp;
    SpotifyPlaylistRegex: RegExp;
    SpotifyArtistRegex: RegExp;
    SpotifyEpisodeRegex: RegExp;
    SpotifyShowRegex: RegExp;
    SpotifyAlbumRegex: RegExp;
    AllSpotifyRegex: RegExp;
    appleMusic: RegExp;
    tidal: RegExp;
    jiosaavn: RegExp;
    PandoraTrackRegex: RegExp;
    PandoraAlbumRegex: RegExp;
    PandoraArtistRegex: RegExp;
    PandoraPlaylistRegex: RegExp;
    AllPandoraRegex: RegExp;
    tiktok: RegExp;
    mixcloud: RegExp;
    musicYandex: RegExp;
    radiohost: RegExp;
};
