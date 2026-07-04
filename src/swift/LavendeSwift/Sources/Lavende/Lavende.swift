import Foundation

public struct TrackInfo: Codable {
  public let title: String
  public let author: String
  public let length: Int64
  public let identifier: String
  public let isStream: Bool
  public let isSeekable: Bool
  public let uri: String
  public let sourceName: String
  public let artworkUrl: String?
  public let issrc: String?
}

public struct Track: Codable {
  public let encoded: String
  public let info: TrackInfo
  public var requester: String?

  public init(encoded: String, info: TrackInfo, requester: String? = nil) {
    self.encoded = encoded
    self.info = info
    self.requester = requester
  }
}

public class Queue {
  public var tracks: [Track] = []
  public var current: Track? = nil
  public var previous: [Track] = []
  public let guildId: String

  public init(guildId: String) {
    self.guildId = guildId
  }

  public var size: Int {
    return tracks.count
  }

  public func add(track: Track, at index: Int? = nil) {
    if let idx = index, idx >= 0, idx <= tracks.count {
      tracks.insert(track, at: idx)
    } else {
      tracks.append(track)
    }
  }

  public func add(tracks newTracks: [Track], at index: Int? = nil) {
    if let idx = index, idx >= 0, idx <= tracks.count {
      tracks.insert(contentsOf: newTracks, at: idx)
    } else {
      tracks.append(contentsOf: newTracks)
    }
  }

  public func remove(at index: Int) {
    if index >= 0 && index < tracks.count {
      tracks.remove(at: index)
    }
  }

  public func clear() {
    tracks.removeAll()
    current = nil
    previous.removeAll()
  }

  public func shuffle() {
    tracks.shuffle()
  }

  public func totalDuration() -> Int64 {
    let currentLen = current?.info.length ?? 0
    let restLen = tracks.reduce(0) { $0 + ($1.info.length) }
    return currentLen + restLen
  }

  public struct QueueUtils {
    private let queue: Queue
    fileprivate init(queue: Queue) { self.queue = queue }

    public func totalDuration() -> Int64 {
      return queue.totalDuration()
    }

    public func filterTracks(predicate: (Track, Int) -> Bool) -> [(track: Track, index: Int)] {
      var results: [(track: Track, index: Int)] = []
      for (index, track) in queue.tracks.enumerated() {
        if predicate(track, index) {
          results.append((track, index))
        }
      }
      return results
    }

    public func findTrack(predicate: (Track, Int) -> Bool) -> (track: Track, index: Int)? {
      return filterTracks(predicate: predicate).first
    }
  }

  public lazy var utils: QueueUtils = {
    return QueueUtils(queue: self)
  }()
}

public class FilterManager {
  public weak var player: Player?
  public var data: [String: Any] = [:]
  public var equalizerBands: [[String: Double]] = []

  public var filters =
    [
      "volume": false,
      "rotation": false,
      "tremolo": false,
      "vibrato": false,
      "lowPass": false,
      "audioOutput": "stereo",
    ] as [String: Any]

  public init(player: Player) {
    self.player = player
  }

  public func applyPlayerFilters() async throws {
    if !equalizerBands.isEmpty {
      data["equalizer"] = equalizerBands
    } else {
      data.removeValue(forKey: "equalizer")
    }

    let jsonData = try JSONSerialization.data(withJSONObject: data, options: [])
    if let jsonString = String(data: jsonData, encoding: .utf8) {
      await player?.setFilters(filtersJson: jsonString)
    }
  }

  public func resetFilters() async throws -> FilterManager {
    data.removeAll()
    equalizerBands.removeAll()
    filters = [
      "volume": false,
      "rotation": false,
      "tremolo": false,
      "vibrato": false,
      "lowPass": false,
      "audioOutput": "stereo",
    ]
    try await applyPlayerFilters()
    return self
  }

  public func setVolume(_ volume: Double) async throws -> FilterManager {
    data["volume"] = volume
    filters["volume"] = volume != 1.0
    try await applyPlayerFilters()
    return self
  }

  public func setAudioOutput(_ type: String) async throws -> FilterManager {
    let mix: [String: Double]
    switch type {
    case "mono":
      mix = ["leftToLeft": 0.5, "leftToRight": 0.5, "rightToLeft": 0.5, "rightToRight": 0.5]
    case "left":
      mix = ["leftToLeft": 1, "leftToRight": 0, "rightToLeft": 1, "rightToRight": 0]
    case "right":
      mix = ["leftToLeft": 0, "leftToRight": 1, "rightToLeft": 0, "rightToRight": 1]
    default:
      mix = ["leftToLeft": 1, "leftToRight": 0, "rightToLeft": 0, "rightToRight": 1]
    }
    data["channelMix"] = mix
    filters["audioOutput"] = type
    try await applyPlayerFilters()
    return self
  }

  public func setSpeed(_ speed: Double = 1.0) async throws -> FilterManager {
    var timescale = data["timescale"] as? [String: Double] ?? [:]
    timescale["speed"] = speed
    data["timescale"] = timescale
    try await applyPlayerFilters()
    return self
  }

  public func setPitch(_ pitch: Double = 1.0) async throws -> FilterManager {
    var timescale = data["timescale"] as? [String: Double] ?? [:]
    timescale["pitch"] = pitch
    data["timescale"] = timescale
    try await applyPlayerFilters()
    return self
  }

  public func setRate(_ rate: Double = 1.0) async throws -> FilterManager {
    var timescale = data["timescale"] as? [String: Double] ?? [:]
    timescale["rate"] = rate
    data["timescale"] = timescale
    try await applyPlayerFilters()
    return self
  }

  public func toggleRotation(rotationHz: Double = 0.2) async throws -> FilterManager {
    if filters["rotation"] as? Bool == true {
      data.removeValue(forKey: "rotation")
    } else {
      data["rotation"] = ["rotationHz": rotationHz]
    }
    filters["rotation"] = !(filters["rotation"] as? Bool ?? false)
    try await applyPlayerFilters()
    return self
  }

  public func toggleVibrato(frequency: Double = 10, depth: Double = 1) async throws -> FilterManager
  {
    if filters["vibrato"] as? Bool == true {
      data.removeValue(forKey: "vibrato")
    } else {
      data["vibrato"] = ["frequency": frequency, "depth": depth]
    }
    filters["vibrato"] = !(filters["vibrato"] as? Bool ?? false)
    try await applyPlayerFilters()
    return self
  }

  public func toggleTremolo(frequency: Double = 4, depth: Double = 0.8) async throws
    -> FilterManager
  {
    if filters["tremolo"] as? Bool == true {
      data.removeValue(forKey: "tremolo")
    } else {
      data["tremolo"] = ["frequency": frequency, "depth": depth]
    }
    filters["tremolo"] = !(filters["tremolo"] as? Bool ?? false)
    try await applyPlayerFilters()
    return self
  }
}

public protocol PlayerDelegate: AnyObject {
  func onTrackStart(player: Player, track: Track)
  func onTrackEnd(player: Player, track: Track, reason: String)
  func onQueueEnd(player: Player)
  func onPlayerDestroy(player: Player, reason: String?)
  func onError(player: Player, error: Error)
}

public enum RepeatMode: String {
  case off, track, queue
}

public struct VoiceState {
  public var sessionId: String?
  public var token: String?
  public var endpoint: String?
}

public class Player {
  public weak var manager: LavendeManager?
  public let player: SwiftLavendePlayer
  public var filterManager: FilterManager!
  public let queue: Queue
  public let guildId: String
  public var voiceChannelId: String?
  public var textChannelId: String?

  public var volume: Double = 100.0
  public var paused: Bool = false
  public var playing: Bool = false
  public var repeatMode: RepeatMode = .off
  public var selfDeaf: Bool = true

  public var voiceState: VoiceState = VoiceState()
  public var node: [String: Any] = [
    "sessionId": "local-session",
    "_checkForSources": false,
    "_checkForPlugins": false,
  ]
  public var playOnConnect: Bool = false
  private var data: [String: Any] = [:]

  public weak var delegate: PlayerDelegate?

  public init(
    manager: LavendeManager, guildId: String, voiceChannelId: String?, textChannelId: String?,
    volume: Double = 100.0, selfDeaf: Bool = true, player: SwiftLavendePlayer
  ) {
    self.manager = manager
    self.guildId = guildId
    self.voiceChannelId = voiceChannelId
    self.textChannelId = textChannelId
    self.volume = volume
    self.selfDeaf = selfDeaf
    self.player = player
    self.queue = Queue(guildId: guildId)
    self.filterManager = FilterManager(player: self)
  }

  public func set(key: String, value: Any) { data[key] = value }
  public func get<T>(key: String) -> T? { return data[key] as? T }
  public func deleteData(key: String) { data.removeValue(forKey: key) }
  public func clearData() { data.removeAll() }
  public func getAllData() -> [String: Any] { return data }

  public func setVoiceState(sessionId: String? = nil, token: String? = nil, endpoint: String? = nil)
  {
    if let s = sessionId { voiceState.sessionId = s }
    if let t = token { voiceState.token = t }
    if let e = endpoint { voiceState.endpoint = e }
    checkPlayOnConnect()
  }

  private func checkPlayOnConnect() {
    if voiceState.sessionId != nil && voiceState.token != nil && voiceState.endpoint != nil
      && playOnConnect
    {
      playOnConnect = false
      Task {
        do {
          try await play()
        } catch {
          delegate?.onError(player: self, error: error)
        }
      }
    }
  }

  public func connect() async {
    await player.connect(channelId: voiceChannelId, selfDeaf: selfDeaf, selfMute: false)
  }

  public func disconnect() async {
    self.voiceChannelId = nil
    await player.disconnect()
    await stop()
  }

  public func destroy(reason: String? = nil) async {
    await disconnect()
    delegate?.onPlayerDestroy(player: self, reason: reason)
    manager?.players.removeValue(forKey: guildId)
    await player.destroy(reason: reason)
  }

  public struct LoadResult: Codable {
    public let loadType: String
    public let tracks: [Track]
    // Could map full LoadResult if needed
  }

  public func search(query: String) async throws -> [Track] {
    let jsonStr = await player.search(query: query)
    // A minimal parser, could be expanded to match load() exactly
    struct SearchResponse: Codable {
      let loadType: String
      let data: [Track]?
    }

    let data = jsonStr.data(using: String.Encoding.utf8)!
    // Quick parse logic
    if let dict = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
      let loadType = dict["loadType"] as? String
    {
      if loadType == "track", let trackData = dict["data"] as? [String: Any] {
        let trackJson = try JSONSerialization.data(withJSONObject: trackData)
        let track = try JSONDecoder().decode(Track.self, from: trackJson)
        return [track]
      } else if loadType == "search", let arr = dict["data"] as? [[String: Any]] {
        let trackJson = try JSONSerialization.data(withJSONObject: arr)
        return try JSONDecoder().decode([Track].self, from: trackJson)
      }
    }
    return []
  }

  public func play(track: Track? = nil, volume: Double? = nil, paused: Bool? = nil) async throws {
    if let t = track {
      queue.current = t
    }
    if let v = volume {
      self.volume = v
    }
    if let p = paused {
      self.paused = p
    }

    if queue.current == nil {
      if queue.tracks.isEmpty {
        self.playing = false
        delegate?.onQueueEnd(player: self)
        return
      }
      queue.current = queue.tracks.removeFirst()
    }

    guard queue.current != nil else {
      throw NSError(
        domain: "Lavende", code: 1,
        userInfo: [NSLocalizedDescriptionKey: "No track available to play"])
    }

    guard voiceState.sessionId != nil, voiceState.token != nil, voiceState.endpoint != nil else {
      playOnConnect = true
      return
    }

    do {
      self.playing = true
      try await player.play()
      if self.paused {
        await player.pause(state: true)
      }
    } catch {
      self.playing = false
      throw error
    }
  }

  public func handleTrackEnd() async throws {
    if let finishedTrack = queue.current {
      if repeatMode == .track {
        try await play()
      } else if repeatMode == .queue {
        queue.add(track: finishedTrack)
        queue.current = nil
        try await play()
      } else {
        queue.previous.append(finishedTrack)
        queue.current = nil
        try await play()
      }
    } else {
      try await play()
    }
  }

  public func pause(_ state: Bool = true) async {
    self.paused = state
    await player.pause(state: state)
  }

  public func resume() async {
    await pause(false)
  }

  public func stop() async {
    self.playing = false
    self.queue.current = nil
    await player.stop()
  }

  public func skip() async {
    await player.stop()
  }

  public func seek(positionMs: Int64) async {
    await player.seek(positionMs: positionMs)
  }

  public func setVolume(_ vol: Double) async {
    self.volume = vol
    await player.setVolume(volume: UInt32(vol / 100.0))
  }

  public func setFilters(filtersJson: String) async {
    await player.setFilters(filtersJson: filtersJson)
  }

  public func getPosition() -> Int64 {
    return player.getPosition()
  }
}

public protocol LavendeManagerDelegate: AnyObject {
  func sendToShard(guildId: String, payloadJson: String)
}

class InternalShardSender: ShardSender {
  weak var manager: LavendeManager?
  func sendToShard(guildId: String, payloadJson: String) {
    manager?.delegate?.sendToShard(guildId: guildId, payloadJson: payloadJson)
  }
}

class InternalEventListener: LavendeEventListener {
  weak var manager: LavendeManager?
  func onEvent(eventJson: String) {
    // Parse eventJson and trigger corresponding Player methods/events
    guard let data = eventJson.data(using: .utf8),
      let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
      let guildId = json["guildId"] as? String,
      let type = json["type"] as? String,
      let player = manager?.players[guildId]
    else { return }

    DispatchQueue.main.async {
      if type == "trackStart" {
        if let track = player.queue.current {
          player.delegate?.onTrackStart(player: player, track: track)
        }
      } else if type == "trackEnd" {
        player.playing = false
        let reason = json["reason"] as? String ?? "FINISHED"
        if let track = player.queue.current {
          player.delegate?.onTrackEnd(player: player, track: track, reason: reason)
        }
        Task {
          do {
            try await player.handleTrackEnd()
          } catch {
            player.delegate?.onError(player: player, error: error)
          }
        }
      }
    }
  }
}

public class LavendeManager: @unchecked Sendable {
  public let clientId: String
  public var players: [String: Player] = [:]
  public weak var delegate: LavendeManagerDelegate?
  private let player: SwiftLavendeManager

  private let internalSender = InternalShardSender()
  private let internalListener = InternalEventListener()

  public init(clientId: String) {
    self.clientId = clientId
    self.player = SwiftLavendeManager(clientId: clientId, shardSender: internalSender)
    self.internalSender.manager = self
    self.internalListener.manager = self

    Task {
      await self.player.listenEvents(listener: self.internalListener)
    }
  }

  public func createPlayer(
    guildId: String, voiceChannelId: String?, textChannelId: String? = nil, volume: Double = 100.0,
    selfDeaf: Bool = true
  ) -> Player {
    if let player = players[guildId] {
      return player
    }
    let innerPlayer = player.getOrCreatePlayer(guildId: guildId)
    let newPlayer = Player(
      manager: self, guildId: guildId, voiceChannelId: voiceChannelId, textChannelId: textChannelId,
      volume: volume, selfDeaf: selfDeaf, player: innerPlayer)
    players[guildId] = newPlayer
    return newPlayer
  }

  public func destroyPlayer(guildId: String) async {
    if let player = players[guildId] {
      await player.destroy()
    }
  }

  public func sendRawData(packet: [String: Any]) {
    guard let t = packet["t"] as? String, let d = packet["d"] as? [String: Any] else { return }

    if t == "VOICE_STATE_UPDATE" {
      if let userId = d["user_id"] as? String, userId == self.clientId {
        if let guildId = d["guild_id"] as? String, let player = players[guildId] {
          player.setVoiceState(sessionId: d["session_id"] as? String)
          player.voiceChannelId = d["channel_id"] as? String
        }
      }
    } else if t == "VOICE_SERVER_UPDATE" {
      if let guildId = d["guild_id"] as? String, let player = players[guildId] {
        player.setVoiceState(token: d["token"] as? String, endpoint: d["endpoint"] as? String)
      }
    }
  }
}
