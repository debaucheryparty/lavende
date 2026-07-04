import { EventEmitter } from "events";

const native = require("../index.js");

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

export class Track {
  public encoded: string;
  public info: TrackInfo;
  public requester: any;

  constructor(data: { encoded: string; info: TrackInfo }, requester: any) {
    this.encoded = data.encoded;
    this.info = data.info;
    this.requester = requester;
  }
}

export class Queue {
  public tracks: Track[] = [];
  public current: Track | null = null;
  public previous: Track[] = [];
  public guildId: string;

  constructor(guildId: string) {
    this.guildId = guildId;
  }

  public get size(): number {
    return this.tracks.length;
  }

  public add(track: Track | Track[], index?: number): void {
    if (Array.isArray(track)) {
      if (typeof index === "number") {
        this.tracks.splice(index, 0, ...track);
      } else {
        this.tracks.push(...track);
      }
    } else {
      if (typeof index === "number") {
        this.tracks.splice(index, 0, track);
      } else {
        this.tracks.push(track);
      }
    }
  }

  public remove(index: number): void {
    if (index >= 0 && index < this.tracks.length) {
      this.tracks.splice(index, 1);
    }
  }

  public clear(): void {
    this.tracks = [];
    this.current = null;
    this.previous = [];
  }

  public shuffle(): void {
    for (let i = this.tracks.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [this.tracks[i], this.tracks[j]] = [this.tracks[j], this.tracks[i]];
    }
  }

  public utils = {
    toJSON: () => {
      return {
        current: this.current ? { ...this.current } : null,
        previous: [...this.previous],
        tracks: [...this.tracks],
      };
    },
    totalDuration: () => {
      return this.tracks.reduce(
        (acc, cur) => acc + (cur.info.length || 0),
        this.current?.info.length || 0,
      );
    },
    filterTracks: (predicate: (track: Track, index: number) => boolean) => {
      return this.tracks
        .map((track, index) => ({ track, index }))
        .filter(({ track, index }) => predicate(track, index));
    },
    findTrack: (predicate: (track: Track, index: number) => boolean) => {
      const results = this.utils.filterTracks(predicate);
      return results.length > 0 ? results[0] : null;
    },
  };
}

export class FilterManager {
  public player: Player;
  public data: any = {};
  public equalizerBands: { band: number; gain: number }[] = [];
  public filters: any = {
    volume: false,
    rotation: false,
    tremolo: false,
    vibrato: false,
    lowPass: false,
    audioOutput: "stereo",
  };

  constructor(player: Player) {
    this.player = player;
  }

  public async applyPlayerFilters(): Promise<void> {
    if (this.equalizerBands.length > 0) {
      this.data.equalizer = this.equalizerBands;
    } else {
      delete this.data.equalizer;
    }
    await this.player.setFilters(this.data);
  }

  public async resetFilters(): Promise<FilterManager> {
    this.data = {};
    this.equalizerBands = [];
    this.filters = {
      volume: false,
      rotation: false,
      tremolo: false,
      vibrato: false,
      lowPass: false,
      audioOutput: "stereo",
    };
    await this.applyPlayerFilters();
    return this;
  }

  public async setVolume(volume: number): Promise<FilterManager> {
    this.data.volume = volume;
    this.filters.volume = volume !== 1.0;
    await this.applyPlayerFilters();
    return this;
  }

  public async setAudioOutput(
    type: "mono" | "stereo" | "left" | "right",
  ): Promise<FilterManager> {
    const mix =
      type === "mono"
        ? {
            leftToLeft: 0.5,
            leftToRight: 0.5,
            rightToLeft: 0.5,
            rightToRight: 0.5,
          }
        : type === "left"
          ? { leftToLeft: 1, leftToRight: 0, rightToLeft: 1, rightToRight: 0 }
          : type === "right"
            ? { leftToLeft: 0, leftToRight: 1, rightToLeft: 0, rightToRight: 1 }
            : {
                leftToLeft: 1,
                leftToRight: 0,
                rightToLeft: 0,
                rightToRight: 1,
              };
    this.data.channelMix = mix;
    this.filters.audioOutput = type;
    await this.applyPlayerFilters();
    return this;
  }

  public async setSpeed(speed: number = 1): Promise<FilterManager> {
    this.data.timescale = { ...this.data.timescale, speed };
    await this.applyPlayerFilters();
    return this;
  }

  public async setPitch(pitch: number = 1): Promise<FilterManager> {
    this.data.timescale = { ...this.data.timescale, pitch };
    await this.applyPlayerFilters();
    return this;
  }

  public async setRate(rate: number = 1): Promise<FilterManager> {
    this.data.timescale = { ...this.data.timescale, rate };
    await this.applyPlayerFilters();
    return this;
  }

  public async toggleRotation(
    rotationHz: number = 0.2,
  ): Promise<FilterManager> {
    if (this.filters.rotation) {
      delete this.data.rotation;
    } else {
      this.data.rotation = { rotationHz };
    }
    this.filters.rotation = !this.filters.rotation;
    await this.applyPlayerFilters();
    return this;
  }

  public async toggleVibrato(
    frequency: number = 10,
    depth: number = 1,
  ): Promise<FilterManager> {
    if (this.filters.vibrato) {
      delete this.data.vibrato;
    } else {
      this.data.vibrato = { frequency, depth };
    }
    this.filters.vibrato = !this.filters.vibrato;
    await this.applyPlayerFilters();
    return this;
  }

  public async toggleTremolo(
    frequency: number = 4,
    depth: number = 0.8,
  ): Promise<FilterManager> {
    if (this.filters.tremolo) {
      delete this.data.tremolo;
    } else {
      this.data.tremolo = { frequency, depth };
    }
    this.filters.tremolo = !this.filters.tremolo;
    await this.applyPlayerFilters();
    return this;
  }
}

export interface PlayerOptions {
  guildId: string;
  voiceChannelId: string;
  textChannelId?: string;
  volume?: number;
  selfDeaf?: boolean;
}

export class Player extends EventEmitter {
  public manager: LavendeManager;
  public filterManager: FilterManager;
  private player: any;
  public queue: Queue;
  public guildId: string;
  public voiceChannelId: string | null = null;
  public textChannelId: string | null = null;
  public volume: number = 100;
  public paused: boolean = false;
  public playing: boolean = false;
  public repeatMode: "off" | "track" | "queue" = "off";
  public selfDeaf: boolean = true;
  public voiceState: {
    sessionId?: string;
    token?: string;
    endpoint?: string;
  } = {};
  public node: any = {
    sessionId: "local-session",
    _checkForSources: false,
    _checkForPlugins: false,
  };

  public playOnConnect: boolean = false;
  private data: Record<string, unknown> = {};

  constructor(manager: LavendeManager, options: PlayerOptions) {
    super();
    this.manager = manager;
    this.guildId = options.guildId;
    this.voiceChannelId = options.voiceChannelId;
    this.textChannelId = options.textChannelId || null;
    this.volume = typeof options.volume === "number" ? options.volume : 100;
    this.selfDeaf = options.selfDeaf !== false;
    this.player = new native.Player(options.guildId);
    this.queue = new Queue(options.guildId);
    this.filterManager = new FilterManager(this);
  }

  public set(key: string, value: unknown) {
    this.data[key] = value;
    return this;
  }

  public get<T>(key: string): T {
    return this.data[key] as T;
  }

  public setData(key: string, value: unknown) {
    this.data[key] = value;
    return this;
  }

  public getData<T>(key: string): T {
    return this.data[key] as T;
  }

  public deleteData(key: string) {
    delete this.data[key];
    return this;
  }

  public clearData() {
    this.data = {};
    return this;
  }

  public getAllData(): Record<string, unknown> {
    return { ...this.data };
  }

  public setVoiceState(state: {
    sessionId?: string;
    token?: string;
    endpoint?: string;
  }) {
    this.voiceState = { ...this.voiceState, ...state };
  }

  public checkPlayOnConnect() {
    const { sessionId, token, endpoint } = this.voiceState;
    if (sessionId && token && endpoint && this.playOnConnect) {
      this.playOnConnect = false;
      console.log(
        `[Player ${this.guildId}] Delayed play handshake completed, starting playback.`,
      );
      process.nextTick(() => {
        this.play().catch((err) => this.emit("error", this, err));
      });
    }
  }

  public async connect(): Promise<void> {
    this.manager.sendToShard(this.guildId, {
      op: 4,
      d: {
        guild_id: this.guildId,
        channel_id: this.voiceChannelId,
        self_mute: false,
        self_deaf: this.selfDeaf,
      },
    });
  }

  public async disconnect(): Promise<void> {
    this.voiceChannelId = null;
    this.manager.sendToShard(this.guildId, {
      op: 4,
      d: {
        guild_id: this.guildId,
        channel_id: null,
        self_mute: false,
        self_deaf: false,
      },
    });
    await this.stop();
  }

  public async destroy(reason?: string): Promise<void> {
    await this.disconnect();
    this.emit("playerDestroy", this, reason);
    this.manager.players.delete(this.guildId);
  }

  public async search(
    query: string | { query: string; source?: string },
    requester: any = null,
  ): Promise<{
    loadType: "track" | "playlist" | "search" | "empty" | "error";
    tracks: Track[];
    playlistInfo?: { name: string; selectedTrack?: number };
    exception?: { message: string; severity: string };
  }> {
    const searchStr = typeof query === "string" ? query : query.query;
    return load(searchStr, requester);
  }

  public async play(options?: {
    track?: Track;
    volume?: number;
    paused?: boolean;
  }): Promise<void> {
    if (options?.track) {
      this.queue.current = options.track;
    }

    if (typeof options?.volume === "number") {
      this.volume = options.volume;
    }

    if (typeof options?.paused === "boolean") {
      this.paused = options.paused;
    }

    if (!this.queue.current) {
      const next = this.queue.tracks.shift();
      if (!next) {
        this.playing = false;
        this.emit("queueEnd", this);
        return;
      }
      this.queue.current = next;
    }

    const currentTrack = this.queue.current;
    if (!currentTrack) {
      throw new Error("No track is currently available to play.");
    }

    const { sessionId, token, endpoint } = this.voiceState;
    if (!sessionId || !token || !endpoint) {
      console.log(
        `[Player ${this.guildId}] Handshake not finished. Queued play for when connected.`,
      );
      this.playOnConnect = true;
      return;
    }

    try {
      this.playing = true;
      const floatVolume = this.volume / 100.0;
      await this.player.setVolume(floatVolume);

      console.log(
        `[Player ${this.guildId}] Invoking native.Player.play with params:`,
        {
          clientId: this.manager.client.id,
          voiceChannelId: this.voiceChannelId,
          sessionId,
          token,
          endpoint,
          identifier: currentTrack.info.uri,
        },
      );

      await this.player.play(
        this.manager.client.id,
        this.voiceChannelId,
        sessionId,
        token,
        endpoint,
        currentTrack.info.uri,
        (err: Error | null, eventJson: string) => {
          if (err) {
            this.emit("error", this, err);
            return;
          }

          try {
            const event = JSON.parse(eventJson);
            const type = event.type;

            if (type === "trackStart") {
              this.emit("trackStart", this, currentTrack);
            } else if (type === "trackEnd") {
              this.playing = false;
              this.emit(
                "trackEnd",
                this,
                currentTrack,
                event.reason || "FINISHED",
              );
              this.handleTrackEnd();
            } else if (type === "position") {
              this.emit("position", this, event.position);
            }
          } catch (parseErr) {
            this.emit("error", this, parseErr);
          }
        },
      );

      if (this.paused) {
        await this.player.pause();
      }
    } catch (err) {
      this.playing = false;
      this.emit("error", this, err);
      throw err;
    }
  }

  private async handleTrackEnd(): Promise<void> {
    const finishedTrack = this.queue.current;
    if (finishedTrack) {
      if (this.repeatMode === "track") {
        await this.play();
      } else if (this.repeatMode === "queue") {
        this.queue.add(finishedTrack);
        this.queue.current = null;
        await this.play();
      } else {
        this.queue.previous.push(finishedTrack);
        this.queue.current = null;
        await this.play();
      }
    } else {
      await this.play();
    }
  }

  public async pause(pauseState: boolean = true): Promise<void> {
    this.paused = pauseState;
    if (pauseState) {
      await this.player.pause();
    } else {
      await this.player.resume();
    }
  }

  public async resume(): Promise<void> {
    await this.pause(false);
  }

  public async stop(): Promise<void> {
    this.playing = false;
    this.queue.current = null;
    await this.player.stop();
  }

  public async skip(): Promise<void> {
    await this.player.stop();
  }

  public async seek(positionMs: number): Promise<void> {
    await this.player.seek(positionMs);
  }

  public async setVolume(volume: number): Promise<void> {
    this.volume = volume;
    const floatVolume = volume / 100.0;
    await this.player.setVolume(floatVolume);
  }

  public setRepeatMode(mode: "off" | "track" | "queue"): void {
    this.repeatMode = mode;
  }

  public async setFilters(filters: any): Promise<void> {
    const jsonStr = JSON.stringify(filters);
    await this.player.setFilters(jsonStr);
  }

  public getPosition(): number {
    return this.player.getPosition();
  }

  public isPaused(): boolean {
    return this.player.isPaused();
  }
}

export interface LavendeManagerOptions {
  sendToShard: (guildId: string, payload: any) => void;
  client: {
    id: string;
    username?: string;
  };
}

export class LavendeManager extends EventEmitter {
  public players = new Map<string, Player>();
  public sendToShard: (guildId: string, payload: any) => void;
  public client: { id: string; username?: string };
  public nodeManager = {
    nodes: new Map(),
  };

  constructor(options: LavendeManagerOptions) {
    super();
    this.sendToShard = options.sendToShard;
    this.client = options.client;
  }

  public init(clientData?: { id: string; username?: string }): void {
    if (clientData) {
      this.client = clientData;
    }
  }

  public createPlayer(options: PlayerOptions): Player {
    let player = this.players.get(options.guildId);
    if (!player) {
      player = new Player(this, options);
      this.players.set(options.guildId, player);
      this.emit("playerCreate", player);

      player.on("trackStart", (p, t) => this.emit("trackStart", p, t));
      player.on("trackEnd", (p, t, r) => this.emit("trackEnd", p, t, r));
      player.on("queueEnd", (p) => this.emit("queueEnd", p));
      player.on("playerDestroy", (p, r) => this.emit("playerDestroy", p, r));
      player.on("error", (p, err) => this.emit("error", p, err));
    }
    return player;
  }

  public destroyPlayer(guildId: string): void {
    const player = this.players.get(guildId);
    if (player) {
      player.destroy();
    }
  }

  public sendRawData(packet: any): void {
    if (!packet || !packet.t) return;

    if (packet.t === "VOICE_STATE_UPDATE") {
      if (packet.d.user_id === this.client.id) {
        const player = this.players.get(packet.d.guild_id);
        if (player) {
          player.setVoiceState({
            sessionId: packet.d.session_id,
          });
          player.voiceChannelId = packet.d.channel_id;
          player.checkPlayOnConnect();
        }
      }
    }

    if (packet.t === "VOICE_SERVER_UPDATE") {
      const player = this.players.get(packet.d.guild_id);
      if (player) {
        player.setVoiceState({
          token: packet.d.token,
          endpoint: packet.d.endpoint,
        });
        player.checkPlayOnConnect();
      }
    }
  }
}

export { Player as LavendePlayer };

export async function load(
  identifier: string,
  requester: any = null,
): Promise<{
  loadType: "track" | "playlist" | "search" | "empty" | "error";
  tracks: Track[];
  playlistInfo?: { name: string; selectedTrack?: number };
  exception?: { message: string; severity: string };
}> {
  const jsonStr = await native.load(identifier);
  const data = JSON.parse(jsonStr);

  const result: any = {
    loadType: "empty",
    tracks: [],
  };

  if (data.loadType === "track") {
    result.loadType = "track";
    result.tracks = [new Track(data.data, requester)];
  } else if (data.loadType === "playlist") {
    result.loadType = "playlist";
    result.playlistInfo = data.data.info;
    result.tracks = data.data.tracks.map((t: any) => new Track(t, requester));
  } else if (data.loadType === "search") {
    result.loadType = "search";
    result.tracks = data.data.map((t: any) => new Track(t, requester));
  } else if (data.loadType === "error") {
    result.loadType = "error";
    result.exception = data.data;
  }

  return result;
}

export class MiniMap<K, V> extends Map<K, V> {
  constructor(data: [K, V][] = []) {
    super(data);
  }

  public filter(
    fn: (value: V, key: K, miniMap: this) => boolean,
  ): MiniMap<K, V> {
    const results = new MiniMap<K, V>();
    for (const [key, val] of this) {
      if (fn(val, key, this)) results.set(key, val);
    }
    return results;
  }

  public map<T>(fn: (value: V, key: K, miniMap: this) => T): T[] {
    const results: T[] = [];
    for (const [key, val] of this) {
      results.push(fn(val, key, this));
    }
    return results;
  }

  public toJSON() {
    return [...this.entries()];
  }
}

export enum DebugEvents {
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
  PlayerChangeNodeFail = "PlayerChangeNodeFail",
}

export enum DestroyReasons {
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
  TrackStuckMaxTracksErroredPerTime = "TrackStuckMaxTracksErroredPerTime",
}

export enum DisconnectReasons {
  Disconnected = "Disconnected",
  DisconnectAllNodes = "DisconnectAllNodes",
}

export const validSponsorBlocks = [
  "sponsor",
  "selfpromo",
  "interaction",
  "intro",
  "outro",
  "preview",
  "music_offtopic",
  "filler",
];

export const audioOutputsData = {
  mono: {
    leftToLeft: 0.5,
    leftToRight: 0.5,
    rightToLeft: 0.5,
    rightToRight: 0.5,
  },
  stereo: {
    leftToLeft: 1,
    leftToRight: 0,
    rightToLeft: 0,
    rightToRight: 1,
  },
  left: {
    leftToLeft: 1,
    leftToRight: 0,
    rightToLeft: 1,
    rightToRight: 0,
  },
  right: {
    leftToLeft: 0,
    leftToRight: 1,
    rightToLeft: 0,
    rightToRight: 1,
  },
};

export const EQList = {
  BassboostEarrape: [
    { band: 0, gain: 0.225 },
    { band: 1, gain: 0.25125 },
    { band: 2, gain: 0.25125 },
    { band: 3, gain: 0.15 },
    { band: 4, gain: -0.1875 },
    { band: 5, gain: 0.05625 },
    { band: 6, gain: -0.16875 },
    { band: 7, gain: 0.08625 },
    { band: 8, gain: 0.13125 },
    { band: 9, gain: 0.16875 },
  ],
};

export const DefaultSources: Record<string, string> = {
  "youtube music": "ytmsearch",
  youtubemusic: "ytmsearch",
  ytmsearch: "ytmsearch",
  ytm: "ytmsearch",
  musicyoutube: "ytmsearch",
  "music youtube": "ytmsearch",
  youtube: "ytsearch",
  yt: "ytsearch",
  ytsearch: "ytsearch",
  soundcloud: "scsearch",
  scsearch: "scsearch",
  sc: "scsearch",
  "apple music": "amsearch",
  apple: "amsearch",
  applemusic: "amsearch",
  amsearch: "amsearch",
  am: "amsearch",
  musicapple: "amsearch",
  "music apple": "amsearch",
  spotify: "spsearch",
  spsearch: "spsearch",
  sp: "spsearch",
  "spotify.com": "spsearch",
  spotifycom: "spsearch",
  sprec: "sprec",
  spsuggestion: "sprec",
  deezer: "dzsearch",
  dz: "dzsearch",
  dzsearch: "dzsearch",
  dzisrc: "dzisrc",
  dzrec: "dzrec",
  "yandex music": "ymsearch",
  yandexmusic: "ymsearch",
  yandex: "ymsearch",
  ymsearch: "ymsearch",
  ymrec: "ymrec",
  vksearch: "vksearch",
  vkmusic: "vksearch",
  "vk music": "vksearch",
  vkrec: "vkrec",
  vk: "vksearch",
  qbsearch: "qbsearch",
  qobuz: "qbsearch",
  qbisrc: "qbisrc",
  qbrec: "qbrec",
  pandora: "pdsearch",
  pd: "pdsearch",
  pdsearch: "pdsearch",
  "pandora music": "pdsearch",
  pandoramusic: "pdsearch",
  speak: "speak",
  tts: "tts",
  ftts: "ftts",
  flowery: "ftts",
  "flowery.tts": "ftts",
  flowerytts: "ftts",
  bandcamp: "bcsearch",
  bc: "bcsearch",
  bcsearch: "bcsearch",
  phsearch: "phsearch",
  pornhub: "phsearch",
  porn: "phsearch",
  local: "local",
  http: "http",
  https: "https",
  link: "link",
  uri: "uri",
  tidal: "tdsearch",
  td: "tdsearch",
  "tidal music": "tdsearch",
  tdrec: "tdrec",
  jiosaavn: "jssearch",
  js: "jssearch",
  jssearch: "jssearch",
  jsrec: "jsrec",
  amzsearch: "amzsearch",
  admsearch: "admsearch",
  gnsearch: "gnsearch",
  szsearch: "szsearch",
};
export const SourceLinksRegexes = {
  YoutubeRegex:
    /https?:\/\/?(?:www\.)?(?:(m|www)\.)?(?:youtu\.be\/|youtube\.com\/(?:embed\/|v\/|shorts|playlist\?|watch\?v=|watch\?.+(?:&|&#38;);v=))([a-zA-Z0-9\-_]{11})?(?:(?:\?|&|&#38;)index=((?:\d){1,3}))?(?:(?:\?|&|&#38;)?list=([a-zA-Z\-_0-9]{34}))?(?:\S+)?/,
  YoutubeMusicRegex:
    /https?:\/\/?(?:www\.)?(?:(music|m|www)\.)?(?:youtu\.be\/|youtube\.com\/(?:embed\/|v\/|shorts|playlist\?|watch\?v=|watch\?.+(?:&|&#38;);v=))([a-zA-Z0-9\-_]{11})?(?:(?:\?|&|&#38;)index=((?:\d){1,3}))?(?:(?:\?|&|&#38;)?list=([a-zA-Z\-_0-9]{34}))?(?:\S+)?/,
  SoundCloudRegex: /https?:\/\/(?:on\.)?soundcloud\.com\//,
  SoundCloudMobileRegex: /https?:\/\/(soundcloud\.app\.goo\.gl)\/(\S+)/,
  bandcamp: /https?:\/\/?(?:www\.)?([\d|\w]+)\.bandcamp\.com\/(\S+)/,
  TwitchTv: /https?:\/\/?(?:www\.)?twitch\.tv\/\w+/,
  vimeo:
    /https?:\/\/(www\.)?vimeo.com\/(?:channels\/(?:\w+\/)?|groups\/([^/]*)\/videos\/|)(\d+)(?:|\/\?)/,
  mp3Url: /(https?|ftp|file):\/\/(www.)?(.*?)\.(mp3)$/,
  m3uUrl: /(https?|ftp|file):\/\/(www.)?(.*?)\.(m3u)$/,
  m3u8Url: /(https?|ftp|file):\/\/(www.)?(.*?)\.(m3u8)$/,
  mp4Url: /(https?|ftp|file):\/\/(www.)?(.*?)\.(mp4)$/,
  m4aUrl: /(https?|ftp|file):\/\/(www.)?(.*?)\.(m4a)$/,
  wavUrl: /(https?|ftp|file):\/\/(www.)?(.*?)\.(wav)$/,
  aacpUrl: /(https?|ftp|file):\/\/(www.)?(.*?)\.(aacp)$/,
  DeezerTrackRegex:
    /(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?track\/(\d+)/,
  DeezerPageLinkRegex: /(https?:\/\/|)?(?:www\.)?deezer\.page\.link\/(\S+)/,
  DeezerPlaylistRegex:
    /(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?playlist\/(\d+)/,
  DeezerAlbumRegex:
    /(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?album\/(\d+)/,
  DeezerArtistRegex:
    /(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?artist\/(\d+)/,
  DeezerMixesRegex:
    /(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?mixes\/genre\/(\d+)/,
  DeezerEpisodeRegex:
    /(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?episode\/(\d+)/,
  AllDeezerRegexWithoutPageLink:
    /(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?(track|playlist|album|artist|mixes\/genre|episode)\/(\d+)/,
  AllDeezerRegex:
    /((https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?(track|playlist|album|artist|mixes\/genre|episode)\/(\d+)|(https?:\/\/|)?(?:www\.)?deezer\.page\.link\/(\S+))/,
  SpotifySongRegex:
    /(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?track\/(?<identifier>[a-zA-Z0-9-_]+)/,
  SpotifyPlaylistRegex:
    /(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?playlist\/(?<identifier>[a-zA-Z0-9-_]+)/,
  SpotifyArtistRegex:
    /(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?artist\/(?<identifier>[a-zA-Z0-9-_]+)/,
  SpotifyEpisodeRegex:
    /(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?episode\/(?<identifier>[a-zA-Z0-9-_]+)/,
  SpotifyShowRegex:
    /(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?show\/(?<identifier>[a-zA-Z0-9-_]+)/,
  SpotifyAlbumRegex:
    /(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?album\/(?<identifier>[a-zA-Z0-9-_]+)/,
  AllSpotifyRegex:
    /(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?(?<type>track|album|playlist|artist|episode|show)\/(?<identifier>[a-zA-Z0-9-_]+)/,
  appleMusic: /https?:\/\/?(?:www\.)?music\.apple\.com\/(\S+)/,
  tidal:
    /https?:\/\/?(?:www\.)?(?:tidal|listen)\.tidal\.com\/(?<type>track|album|playlist|artist)\/(?<identifier>[a-zA-Z0-9-_]+)/,
  jiosaavn:
    /(https?:\/\/)(www\.)?jiosaavn\.com\/(?<type>song|album|featured|artist)\/([a-zA-Z0-9-_/,]+)/,
  PandoraTrackRegex:
    /^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/artist\/[\w-]+(?:\/[\w-]+)*\/(?<identifier>TR[A-Za-z0-9]+)(?:[?#].*)?$/,
  PandoraAlbumRegex:
    /^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/artist\/[\w-]+(?:\/[\w-]+)*\/(?<identifier>AL[A-Za-z0-9]+)(?:[?#].*)?$/,
  PandoraArtistRegex:
    /^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/artist\/[\w-]+\/(?<identifier>AR[A-Za-z0-9]+)(?:[?#].*)?$/,
  PandoraPlaylistRegex:
    /^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/playlist\/(?<identifier>PL:[\d:]+)(?:[?#].*)?$/,
  AllPandoraRegex:
    /^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/(?:playlist\/(?<playlistId>PL:[\d:]+)|artist\/[\w-]+(?:\/[\w-]+)*\/(?<identifier>(?:TR|AL|AR)[A-Za-z0-9]+))(?:[?#].*)?$/,
  tiktok: /https:\/\/www\.tiktok\.com\//,
  mixcloud: /https:\/\/www\.mixcloud\.com\//,
  musicYandex: /https:\/\/music\.yandex\.ru\//,
  radiohost: /https?:\/\/[^.\s]+\.radiohost\.de\/(\S+)/,
};
