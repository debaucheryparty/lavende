package lavende

/*
#cgo CFLAGS: -I.
#cgo LDFLAGS: -L. -llavende_go -lm -ldl -lpthread
#if defined(__APPLE__)
#cgo LDFLAGS: -framework Security -framework CoreFoundation -framework SystemConfiguration
#endif

#include "lavende.h"
#include <stdlib.h>

extern void goSendToShard(char* guildId, char* payloadJson);
extern void goOnEvent(char* eventJson);

static void c_send_to_shard(const char* guild_id, const char* payload_json) {
    goSendToShard((char*)guild_id, (char*)payload_json);
}

static void c_on_event(const char* event_json) {
    goOnEvent((char*)event_json);
}

static void* get_send_to_shard_cb() {
    return (void*)c_send_to_shard;
}

static void* get_on_event_cb() {
    return (void*)c_on_event;
}
*/
import "C"
import (
	"encoding/json"
	"errors"
	"math/rand"
	"sync"
	"unsafe"
)

var (
	sendToShardFunc func(guildId string, payload json.RawMessage)
	onEventFunc     func(event json.RawMessage)
)

//export goSendToShard
func goSendToShard(guildId *C.char, payloadJson *C.char) {
	if sendToShardFunc != nil {
		gId := C.GoString(guildId)
		pJson := C.GoString(payloadJson)
		sendToShardFunc(gId, json.RawMessage(pJson))
	}
}

//export goOnEvent
func goOnEvent(eventJson *C.char) {
	if onEventFunc != nil {
		eJson := C.GoString(eventJson)
		onEventFunc(json.RawMessage(eJson))
	}
}

type TrackInfo struct {
	Title      string  `json:"title"`
	Author     string  `json:"author"`
	Length     int64   `json:"length"`
	Identifier string  `json:"identifier"`
	IsStream   bool    `json:"isStream"`
	IsSeekable bool    `json:"isSeekable"`
	Uri        string  `json:"uri"`
	SourceName string  `json:"sourceName"`
	ArtworkUrl *string `json:"artworkUrl,omitempty"`
	Isrc       *string `json:"isrc,omitempty"`
}

type TrackData struct {
	Encoded string    `json:"encoded"`
	Info    TrackInfo `json:"info"`
}

type Track struct {
	Encoded   string
	Info      TrackInfo
	Requester interface{}
}

func NewTrack(data TrackData, requester interface{}) *Track {
	return &Track{
		Encoded:   data.Encoded,
		Info:      data.Info,
		Requester: requester,
	}
}

type QueueUtils struct {
	q *Queue
}

func (u *QueueUtils) ToJSON() map[string]interface{} {
	var current interface{}
	if u.q.Current != nil {
		current = *u.q.Current
	}
	return map[string]interface{}{
		"current":  current,
		"previous": u.q.Previous,
		"tracks":   u.q.Tracks,
	}
}

func (u *QueueUtils) TotalDuration() int64 {
	var total int64
	if u.q.Current != nil {
		total += u.q.Current.Info.Length
	}
	for _, t := range u.q.Tracks {
		total += t.Info.Length
	}
	return total
}

func (u *QueueUtils) FilterTracks(predicate func(*Track, int) bool) []struct {
	Track *Track
	Index int
} {
	var results []struct {
		Track *Track
		Index int
	}
	for i, t := range u.q.Tracks {
		if predicate(t, i) {
			results = append(results, struct {
				Track *Track
				Index int
			}{t, i})
		}
	}
	return results
}

func (u *QueueUtils) FindTrack(predicate func(*Track, int) bool) *struct {
	Track *Track
	Index int
} {
	results := u.FilterTracks(predicate)
	if len(results) > 0 {
		return &results[0]
	}
	return nil
}

type Queue struct {
	Tracks   []*Track
	Current  *Track
	Previous []*Track
	GuildId  string
	Utils    *QueueUtils
}

func NewQueue(guildId string) *Queue {
	q := &Queue{
		Tracks:   make([]*Track, 0),
		Current:  nil,
		Previous: make([]*Track, 0),
		GuildId:  guildId,
	}
	q.Utils = &QueueUtils{q: q}
	return q
}

func (q *Queue) Size() int {
	return len(q.Tracks)
}

func (q *Queue) AddSingle(track *Track, index *int) {
	if index != nil {
		q.Tracks = append(q.Tracks[:*index], append([]*Track{track}, q.Tracks[*index:]...)...)
	} else {
		q.Tracks = append(q.Tracks, track)
	}
}

func (q *Queue) AddMultiple(tracks []*Track, index *int) {
	if index != nil {
		q.Tracks = append(q.Tracks[:*index], append(tracks, q.Tracks[*index:]...)...)
	} else {
		q.Tracks = append(q.Tracks, tracks...)
	}
}

func (q *Queue) Remove(index int) {
	if index >= 0 && index < len(q.Tracks) {
		q.Tracks = append(q.Tracks[:index], q.Tracks[index+1:]...)
	}
}

func (q *Queue) Clear() {
	q.Tracks = make([]*Track, 0)
	q.Current = nil
	q.Previous = make([]*Track, 0)
}

func (q *Queue) Shuffle() {
	for i := len(q.Tracks) - 1; i > 0; i-- {
		j := rand.Intn(i + 1)
		q.Tracks[i], q.Tracks[j] = q.Tracks[j], q.Tracks[i]
	}
}

type EqBand struct {
	Band int     `json:"band"`
	Gain float64 `json:"gain"`
}

type FilterManager struct {
	Player         *Player
	Data           map[string]interface{}
	EqualizerBands []EqBand
	Filters        map[string]interface{}
}

func NewFilterManager(player *Player) *FilterManager {
	return &FilterManager{
		Player:         player,
		Data:           make(map[string]interface{}),
		EqualizerBands: make([]EqBand, 0),
		Filters: map[string]interface{}{
			"volume":      false,
			"rotation":    false,
			"tremolo":     false,
			"vibrato":     false,
			"lowPass":     false,
			"audioOutput": "stereo",
		},
	}
}

func (f *FilterManager) ApplyPlayerFilters() error {
	if len(f.EqualizerBands) > 0 {
		f.Data["equalizer"] = f.EqualizerBands
	} else {
		delete(f.Data, "equalizer")
	}
	return f.Player.SetFilters(f.Data)
}

func (f *FilterManager) ResetFilters() (*FilterManager, error) {
	f.Data = make(map[string]interface{})
	f.EqualizerBands = make([]EqBand, 0)
	f.Filters = map[string]interface{}{
		"volume":      false,
		"rotation":    false,
		"tremolo":     false,
		"vibrato":     false,
		"lowPass":     false,
		"audioOutput": "stereo",
	}
	err := f.ApplyPlayerFilters()
	return f, err
}

func (f *FilterManager) SetVolume(volume float64) (*FilterManager, error) {
	f.Data["volume"] = volume
	f.Filters["volume"] = volume != 1.0
	err := f.ApplyPlayerFilters()
	return f, err
}

func (f *FilterManager) SetAudioOutput(typ string) (*FilterManager, error) {
	var mix map[string]float64
	switch typ {
	case "mono":
		mix = map[string]float64{"leftToLeft": 0.5, "leftToRight": 0.5, "rightToLeft": 0.5, "rightToRight": 0.5}
	case "left":
		mix = map[string]float64{"leftToLeft": 1, "leftToRight": 0, "rightToLeft": 1, "rightToRight": 0}
	case "right":
		mix = map[string]float64{"leftToLeft": 0, "leftToRight": 1, "rightToLeft": 0, "rightToRight": 1}
	default:
		mix = map[string]float64{"leftToLeft": 1, "leftToRight": 0, "rightToLeft": 0, "rightToRight": 1}
	}
	f.Data["channelMix"] = mix
	f.Filters["audioOutput"] = typ
	err := f.ApplyPlayerFilters()
	return f, err
}

func (f *FilterManager) SetSpeed(speed float64) (*FilterManager, error) {
	ts, ok := f.Data["timescale"].(map[string]float64)
	if !ok {
		ts = make(map[string]float64)
	}
	ts["speed"] = speed
	f.Data["timescale"] = ts
	err := f.ApplyPlayerFilters()
	return f, err
}

func (f *FilterManager) SetPitch(pitch float64) (*FilterManager, error) {
	ts, ok := f.Data["timescale"].(map[string]float64)
	if !ok {
		ts = make(map[string]float64)
	}
	ts["pitch"] = pitch
	f.Data["timescale"] = ts
	err := f.ApplyPlayerFilters()
	return f, err
}

func (f *FilterManager) SetRate(rate float64) (*FilterManager, error) {
	ts, ok := f.Data["timescale"].(map[string]float64)
	if !ok {
		ts = make(map[string]float64)
	}
	ts["rate"] = rate
	f.Data["timescale"] = ts
	err := f.ApplyPlayerFilters()
	return f, err
}

func (f *FilterManager) ToggleRotation(rotationHz float64) (*FilterManager, error) {
	if f.Filters["rotation"].(bool) {
		delete(f.Data, "rotation")
	} else {
		f.Data["rotation"] = map[string]float64{"rotationHz": rotationHz}
	}
	f.Filters["rotation"] = !f.Filters["rotation"].(bool)
	err := f.ApplyPlayerFilters()
	return f, err
}

func (f *FilterManager) ToggleVibrato(frequency float64, depth float64) (*FilterManager, error) {
	if f.Filters["vibrato"].(bool) {
		delete(f.Data, "vibrato")
	} else {
		f.Data["vibrato"] = map[string]float64{"frequency": frequency, "depth": depth}
	}
	f.Filters["vibrato"] = !f.Filters["vibrato"].(bool)
	err := f.ApplyPlayerFilters()
	return f, err
}

func (f *FilterManager) ToggleTremolo(frequency float64, depth float64) (*FilterManager, error) {
	if f.Filters["tremolo"].(bool) {
		delete(f.Data, "tremolo")
	} else {
		f.Data["tremolo"] = map[string]float64{"frequency": frequency, "depth": depth}
	}
	f.Filters["tremolo"] = !f.Filters["tremolo"].(bool)
	err := f.ApplyPlayerFilters()
	return f, err
}

type PlayerOptions struct {
	GuildId        string
	VoiceChannelId string
	TextChannelId  *string
	Volume         *int
	SelfDeaf       *bool
}

type EventEmitter struct {
	mu        sync.RWMutex
	listeners map[string][]func(args ...interface{})
}

func NewEventEmitter() *EventEmitter {
	return &EventEmitter{
		listeners: make(map[string][]func(args ...interface{})),
	}
}

func (e *EventEmitter) On(event string, listener func(args ...interface{})) {
	e.mu.Lock()
	defer e.mu.Unlock()
	e.listeners[event] = append(e.listeners[event], listener)
}

func (e *EventEmitter) Emit(event string, args ...interface{}) {
	e.mu.RLock()
	defer e.mu.RUnlock()
	for _, l := range e.listeners[event] {
		l(args...)
	}
}

type VoiceState struct {
	SessionId *string
	Token     *string
	Endpoint  *string
}

type Player struct {
	*EventEmitter
	Manager        *LavendeManager
	FilterManager  *FilterManager
	ptr            *C.LavendePlayer
	Queue          *Queue
	GuildId        string
	VoiceChannelId *string
	TextChannelId  *string
	Volume         int
	Paused         bool
	Playing        bool
	RepeatMode     string
	SelfDeaf       bool
	VoiceState     VoiceState
	Node           map[string]interface{}
	PlayOnConnect  bool
	data           map[string]interface{}
}

func NewPlayer(manager *LavendeManager, options PlayerOptions) *Player {
	vol := 100
	if options.Volume != nil {
		vol = *options.Volume
	}
	sd := true
	if options.SelfDeaf != nil {
		sd = *options.SelfDeaf
	}

	cGuildId := C.CString(options.GuildId)
	defer C.free(unsafe.Pointer(cGuildId))
	ptr := C.lavende_manager_get_or_create_player(manager.ptr, cGuildId)

	p := &Player{
		EventEmitter:   NewEventEmitter(),
		Manager:        manager,
		ptr:            ptr,
		GuildId:        options.GuildId,
		VoiceChannelId: &options.VoiceChannelId,
		TextChannelId:  options.TextChannelId,
		Volume:         vol,
		Paused:         false,
		Playing:        false,
		RepeatMode:     "off",
		SelfDeaf:       sd,
		VoiceState:     VoiceState{},
		Node: map[string]interface{}{
			"sessionId":        "local-session",
			"_checkForSources": false,
			"_checkForPlugins": false,
		},
		PlayOnConnect: false,
		data:          make(map[string]interface{}),
	}
	p.Queue = NewQueue(options.GuildId)
	p.FilterManager = NewFilterManager(p)
	return p
}

func (p *Player) Set(key string, value interface{}) *Player {
	p.data[key] = value
	return p
}

func (p *Player) Get(key string) interface{} {
	return p.data[key]
}

func (p *Player) SetData(key string, value interface{}) *Player {
	p.data[key] = value
	return p
}

func (p *Player) GetData(key string) interface{} {
	return p.data[key]
}

func (p *Player) DeleteData(key string) *Player {
	delete(p.data, key)
	return p
}

func (p *Player) ClearData() *Player {
	p.data = make(map[string]interface{})
	return p
}

func (p *Player) GetAllData() map[string]interface{} {
	res := make(map[string]interface{})
	for k, v := range p.data {
		res[k] = v
	}
	return res
}

func (p *Player) SetVoiceState(state VoiceState) {
	if state.SessionId != nil {
		p.VoiceState.SessionId = state.SessionId
	}
	if state.Token != nil {
		p.VoiceState.Token = state.Token
	}
	if state.Endpoint != nil {
		p.VoiceState.Endpoint = state.Endpoint
	}
}

func (p *Player) CheckPlayOnConnect() {
	if p.VoiceState.SessionId != nil && p.VoiceState.Token != nil && p.VoiceState.Endpoint != nil && p.PlayOnConnect {
		p.PlayOnConnect = false
		go func() {
			err := p.Play(nil)
			if err != nil {
				p.Emit("error", p, err)
			}
		}()
	}
}

func (p *Player) Connect() error {
	payload := map[string]interface{}{
		"op": 4,
		"d": map[string]interface{}{
			"guild_id":   p.GuildId,
			"channel_id": p.VoiceChannelId,
			"self_mute":  false,
			"self_deaf":  p.SelfDeaf,
		},
	}
	p.Manager.SendToShard(p.GuildId, payload)

	var cChannelId *C.char
	if p.VoiceChannelId != nil {
		cChannelId = C.CString(*p.VoiceChannelId)
		defer C.free(unsafe.Pointer(cChannelId))
	}
	C.lavende_player_connect(p.ptr, cChannelId, C.bool(p.SelfDeaf), C.bool(false))
	return nil
}

func (p *Player) Disconnect() error {
	p.VoiceChannelId = nil
	payload := map[string]interface{}{
		"op": 4,
		"d": map[string]interface{}{
			"guild_id":   p.GuildId,
			"channel_id": nil,
			"self_mute":  false,
			"self_deaf":  false,
		},
	}
	p.Manager.SendToShard(p.GuildId, payload)
	C.lavende_player_disconnect(p.ptr)
	return p.Stop()
}

func (p *Player) Destroy(reason *string) error {
	p.Disconnect()
	p.Emit("playerDestroy", p, reason)
	p.Manager.Players.Delete(p.GuildId)

	var cReason *C.char
	if reason != nil {
		cReason = C.CString(*reason)
		defer C.free(unsafe.Pointer(cReason))
	}
	C.lavende_player_destroy(p.ptr, cReason)
	C.lavende_player_free(p.ptr)
	p.ptr = nil
	return nil
}

type LoadResult struct {
	LoadType     string      `json:"loadType"`
	Tracks       []*Track    `json:"tracks"`
	PlaylistInfo interface{} `json:"playlistInfo,omitempty"`
	Exception    interface{} `json:"exception,omitempty"`
}

func (p *Player) Search(query string, requester interface{}) (*LoadResult, error) {
	cQuery := C.CString(query)
	defer C.free(unsafe.Pointer(cQuery))

	cRes := C.lavende_player_search(p.ptr, cQuery)
	if cRes == nil {
		return nil, errors.New("search returned nil")
	}
	defer C.lavende_free_string(cRes)

	resStr := C.GoString(cRes)

	var raw struct {
		LoadType string          `json:"loadType"`
		Data     json.RawMessage `json:"data"`
	}
	if err := json.Unmarshal([]byte(resStr), &raw); err != nil {
		return nil, err
	}

	res := &LoadResult{
		LoadType: raw.LoadType,
		Tracks:   make([]*Track, 0),
	}

	if raw.LoadType == "track" {
		var td TrackData
		json.Unmarshal(raw.Data, &td)
		res.Tracks = append(res.Tracks, NewTrack(td, requester))
	} else if raw.LoadType == "playlist" {
		var pd struct {
			Info   interface{} `json:"info"`
			Tracks []TrackData `json:"tracks"`
		}
		json.Unmarshal(raw.Data, &pd)
		res.PlaylistInfo = pd.Info
		for _, td := range pd.Tracks {
			res.Tracks = append(res.Tracks, NewTrack(td, requester))
		}
	} else if raw.LoadType == "search" {
		var tds []TrackData
		json.Unmarshal(raw.Data, &tds)
		for _, td := range tds {
			res.Tracks = append(res.Tracks, NewTrack(td, requester))
		}
	} else if raw.LoadType == "error" {
		var e interface{}
		json.Unmarshal(raw.Data, &e)
		res.Exception = e
	}

	return res, nil
}

type PlayOptions struct {
	Track  *Track
	Volume *int
	Paused *bool
}

func (p *Player) Play(options *PlayOptions) error {
	if options != nil && options.Track != nil {
		p.Queue.Current = options.Track
	}
	if options != nil && options.Volume != nil {
		p.Volume = *options.Volume
	}
	if options != nil && options.Paused != nil {
		p.Paused = *options.Paused
	}

	if p.Queue.Current == nil {
		if len(p.Queue.Tracks) > 0 {
			p.Queue.Current = p.Queue.Tracks[0]
			p.Queue.Tracks = p.Queue.Tracks[1:]
		} else {
			p.Playing = false
			p.Emit("queueEnd", p)
			return nil
		}
	}

	if p.VoiceState.SessionId == nil || p.VoiceState.Token == nil || p.VoiceState.Endpoint == nil {
		p.PlayOnConnect = true
		return nil
	}

	p.Playing = true
	C.lavende_player_set_volume(p.ptr, C.uint32_t(p.Volume))

	cErr := C.lavende_player_play(p.ptr)
	if cErr != nil {
		defer C.lavende_free_string(cErr)
		err := errors.New(C.GoString(cErr))
		p.Playing = false
		p.Emit("error", p, err)
		return err
	}

	if p.Paused {
		C.lavende_player_pause(p.ptr, C.bool(true))
	}
	return nil
}

func (p *Player) HandleTrackEnd() {
	finishedTrack := p.Queue.Current
	if finishedTrack != nil {
		if p.RepeatMode == "track" {
			p.Play(nil)
		} else if p.RepeatMode == "queue" {
			p.Queue.AddSingle(finishedTrack, nil)
			p.Queue.Current = nil
			p.Play(nil)
		} else {
			p.Queue.Previous = append(p.Queue.Previous, finishedTrack)
			p.Queue.Current = nil
			p.Play(nil)
		}
	} else {
		p.Play(nil)
	}
}

func (p *Player) Pause(state bool) error {
	p.Paused = state
	C.lavende_player_pause(p.ptr, C.bool(state))
	return nil
}

func (p *Player) Resume() error {
	return p.Pause(false)
}

func (p *Player) Stop() error {
	p.Playing = false
	p.Queue.Current = nil
	C.lavende_player_stop(p.ptr)
	return nil
}

func (p *Player) Skip() error {
	C.lavende_player_skip(p.ptr)
	return nil
}

func (p *Player) Seek(positionMs int64) error {
	C.lavende_player_seek(p.ptr, C.int64_t(positionMs))
	return nil
}

func (p *Player) SetVolumeObj(volume int) error {
	p.Volume = volume
	C.lavende_player_set_volume(p.ptr, C.uint32_t(volume))
	return nil
}

func (p *Player) SetRepeatMode(mode string) {
	p.RepeatMode = mode
}

func (p *Player) SetFilters(filters map[string]interface{}) error {
	b, _ := json.Marshal(filters)
	cJson := C.CString(string(b))
	defer C.free(unsafe.Pointer(cJson))
	C.lavende_player_set_filters(p.ptr, cJson)
	return nil
}

func (p *Player) GetPosition() int64 {
	return int64(C.lavende_player_get_position(p.ptr))
}

func (p *Player) IsPaused() bool {
	return bool(C.lavende_player_is_paused(p.ptr))
}

type LavendeManagerOptions struct {
	SendToShard func(guildId string, payload interface{})
	Client      struct {
		Id       string
		Username *string
	}
}

type LavendeManager struct {
	*EventEmitter
	Players     sync.Map
	SendToShard func(guildId string, payload interface{})
	Client      struct {
		Id       string
		Username *string
	}
	NodeManager struct {
		Nodes sync.Map
	}
	ptr *C.LavendeManager
}

func NewLavendeManager(options LavendeManagerOptions) *LavendeManager {
	m := &LavendeManager{
		EventEmitter: NewEventEmitter(),
		SendToShard:  options.SendToShard,
		Client:       options.Client,
	}
	sendToShardFunc = func(guildId string, payload json.RawMessage) {
		var p interface{}
		json.Unmarshal(payload, &p)
		m.SendToShard(guildId, p)
	}
	onEventFunc = func(event json.RawMessage) {
		var ev map[string]interface{}
		json.Unmarshal(event, &ev)
		if typ, ok := ev["type"].(string); ok {
			if guildId, ok := ev["guildId"].(string); ok {
				if pi, ok := m.Players.Load(guildId); ok {
					player := pi.(*Player)
					if typ == "trackStart" {
						player.Emit("trackStart", player, player.Queue.Current)
					} else if typ == "trackEnd" {
						player.Playing = false
						reason := "FINISHED"
						if r, ok := ev["reason"].(string); ok {
							reason = r
						}
						player.Emit("trackEnd", player, player.Queue.Current, reason)
						player.HandleTrackEnd()
					} else if typ == "position" {
						player.Emit("position", player, ev["position"])
					}
				}
			}
		}
	}

	cClientId := C.CString(options.Client.Id)
	defer C.free(unsafe.Pointer(cClientId))
	cb := (C.SendToShardCb)(C.get_send_to_shard_cb())
	m.ptr = C.lavende_manager_new(cClientId, cb)

	cbe := (C.EventCb)(C.get_on_event_cb())
	C.lavende_manager_listen_events(m.ptr, cbe)

	return m
}

func (m *LavendeManager) Init(clientData *struct {
	Id       string
	Username *string
}) {
	if clientData != nil {
		m.Client = *clientData
	}
}

func (m *LavendeManager) CreatePlayer(options PlayerOptions) *Player {
	if pi, ok := m.Players.Load(options.GuildId); ok {
		return pi.(*Player)
	}
	player := NewPlayer(m, options)
	m.Players.Store(options.GuildId, player)
	m.Emit("playerCreate", player)

	player.On("trackStart", func(args ...interface{}) { m.Emit("trackStart", args...) })
	player.On("trackEnd", func(args ...interface{}) { m.Emit("trackEnd", args...) })
	player.On("queueEnd", func(args ...interface{}) { m.Emit("queueEnd", args...) })
	player.On("playerDestroy", func(args ...interface{}) { m.Emit("playerDestroy", args...) })
	player.On("error", func(args ...interface{}) { m.Emit("error", args...) })

	return player
}

func (m *LavendeManager) DestroyPlayer(guildId string) {
	if pi, ok := m.Players.Load(guildId); ok {
		pi.(*Player).Destroy(nil)
	}
}

func (m *LavendeManager) SendRawData(packet map[string]interface{}) {
	t, _ := packet["t"].(string)
	d, _ := packet["d"].(map[string]interface{})
	if t == "" || d == nil {
		return
	}

	if t == "VOICE_STATE_UPDATE" {
		if d["user_id"] == m.Client.Id {
			if pi, ok := m.Players.Load(d["guild_id"].(string)); ok {
				player := pi.(*Player)
				sid, _ := d["session_id"].(string)
				player.SetVoiceState(VoiceState{SessionId: &sid})
				cid, _ := d["channel_id"].(string)
				player.VoiceChannelId = &cid
				player.CheckPlayOnConnect()
			}
		}
	}

	if t == "VOICE_SERVER_UPDATE" {
		if pi, ok := m.Players.Load(d["guild_id"].(string)); ok {
			player := pi.(*Player)
			tok, _ := d["token"].(string)
			endp, _ := d["endpoint"].(string)
			player.SetVoiceState(VoiceState{Token: &tok, Endpoint: &endp})
			player.CheckPlayOnConnect()
		}
	}
}

type MiniMap struct {
	sync.Map
}

func (m *MiniMap) Filter(fn func(value interface{}, key interface{}) bool) *MiniMap {
	res := &MiniMap{}
	m.Range(func(key, value interface{}) bool {
		if fn(value, key) {
			res.Store(key, value)
		}
		return true
	})
	return res
}

func (m *MiniMap) ToJSON() []map[string]interface{} {
	var res []map[string]interface{}
	m.Range(func(key, value interface{}) bool {
		res = append(res, map[string]interface{}{"key": key, "value": value})
		return true
	})
	return res
}

const (
	DebugSetSponsorBlock                    = "SetSponsorBlock"
	DebugDeleteSponsorBlock                 = "DeleteSponsorBlock"
	DebugTrackEndReplaced                   = "TrackEndReplaced"
	DebugAutoplayExecution                  = "AutoplayExecution"
	DebugAutoplayNoSongsAdded               = "AutoplayNoSongsAdded"
	DebugAutoplayThresholdSpamLimiter       = "AutoplayThresholdSpamLimiter"
	DebugTriggerQueueEmptyInterval          = "TriggerQueueEmptyInterval"
	DebugQueueEnded                         = "QueueEnded"
	DebugTrackStartNewSongsOnly             = "TrackStartNewSongsOnly"
	DebugTrackStartNoTrack                  = "TrackStartNoTrack"
	DebugResumingFetchingError              = "ResumingFetchingError"
	DebugPlayerUpdateNoPlayer               = "PlayerUpdateNoPlayer"
	DebugPlayerUpdateFilterFixApply         = "PlayerUpdateFilterFixApply"
	DebugPlayerUpdateSuccess                = "PlayerUpdateSuccess"
	DebugHeartBeatTriggered                 = "HeartBeatTriggered"
	DebugNoSocketOnDestroy                  = "NoSocketOnDestroy"
	DebugSocketCleanupError                 = "SocketCleanupError"
	DebugSocketTerminateHeartBeatTimeout    = "SocketTerminateHeartBeatTimeout"
	DebugTryingConnectWhileConnected        = "TryingConnectWhileConnected"
	DebugLavaSearchNothingFound             = "LavaSearchNothingFound"
	DebugSearchNothingFound                 = "SearchNothingFound"
	DebugValidatingBlacklistLinks           = "ValidatingBlacklistLinks"
	DebugValidatingWhitelistLinks           = "ValidatingWhitelistLinks"
	DebugTrackErrorMaxTracksErroredPerTime  = "TrackErrorMaxTracksErroredPerTime"
	DebugTrackStuckMaxTracksErroredPerTime  = "TrackStuckMaxTracksErroredPerTime"
	DebugPlayerDestroyingSomewhereElse      = "PlayerDestroyingSomewhereElse"
	DebugPlayerCreateNodeNotFound           = "PlayerCreateNodeNotFound"
	DebugPlayerPlayQueueEmptyTimeoutClear   = "PlayerPlayQueueEmptyTimeoutClear"
	DebugPlayerPlayWithTrackReplace         = "PlayerPlayWithTrackReplace"
	DebugPlayerPlayUnresolvedTrack          = "PlayerPlayUnresolvedTrack"
	DebugPlayerPlayUnresolvedTrackFailed    = "PlayerPlayUnresolvedTrackFailed"
	DebugPlayerVolumeAsFilter               = "PlayerVolumeAsFilter"
	DebugBandcampSearchLokalEngine          = "BandcampSearchLokalEngine"
	DebugPlayerChangeNode                   = "PlayerChangeNode"
	DebugBuildTrackError                    = "BuildTrackError"
	DebugTransformRequesterFunctionFailed   = "TransformRequesterFunctionFailed"
	DebugGetClosestTrackFailed              = "GetClosestTrackFailed"
	DebugPlayerDeleteInsteadOfDestroy       = "PlayerDeleteInsteadOfDestroy"
	DebugFailedToConnectToNodes             = "FailedToConnectToNodes"
	DebugNoAudioDebug                       = "NoAudioDebug"
	DebugPlayerAutoReconnect                = "PlayerAutoReconnect"
	DebugPlayerDestroyFail                  = "PlayerDestroyFail"
	DebugPlayerChangeNodeFailNoEligibleNode = "PlayerChangeNodeFailNoEligibleNode"
	DebugPlayerChangeNodeFail               = "PlayerChangeNodeFail"
)

const (
	DestroyReasonQueueEmpty                         = "QueueEmpty"
	DestroyReasonNodeDestroy                        = "NodeDestroy"
	DestroyReasonNodeDeleted                        = "NodeDeleted"
	DestroyReasonLavalinkNoVoice                    = "LavalinkNoVoice"
	DestroyReasonNodeReconnectFail                  = "NodeReconnectFail"
	DestroyReasonDisconnected                       = "Disconnected"
	DestroyReasonPlayerReconnectFail                = "PlayerReconnectFail"
	DestroyReasonPlayerChangeNodeFail               = "PlayerChangeNodeFail"
	DestroyReasonPlayerChangeNodeFailNoEligibleNode = "PlayerChangeNodeFailNoEligibleNode"
	DestroyReasonChannelDeleted                     = "ChannelDeleted"
	DestroyReasonDisconnectAllNodes                 = "DisconnectAllNodes"
	DestroyReasonReconnectAllNodes                  = "ReconnectAllNodes"
	DestroyReasonTrackErrorMaxTracksErroredPerTime  = "TrackErrorMaxTracksErroredPerTime"
	DestroyReasonTrackStuckMaxTracksErroredPerTime  = "TrackStuckMaxTracksErroredPerTime"
)

const (
	DisconnectReasonDisconnected       = "Disconnected"
	DisconnectReasonDisconnectAllNodes = "DisconnectAllNodes"
)

var ValidSponsorBlocks = []string{
	"sponsor",
	"selfpromo",
	"interaction",
	"intro",
	"outro",
	"preview",
	"music_offtopic",
	"filler",
}

var AudioOutputsData = map[string]map[string]float64{
	"mono": {
		"leftToLeft":   0.5,
		"leftToRight":  0.5,
		"rightToLeft":  0.5,
		"rightToRight": 0.5,
	},
	"stereo": {
		"leftToLeft":   1,
		"leftToRight":  0,
		"rightToLeft":  0,
		"rightToRight": 1,
	},
	"left": {
		"leftToLeft":   1,
		"leftToRight":  0,
		"rightToLeft":  1,
		"rightToRight": 0,
	},
	"right": {
		"leftToLeft":   0,
		"leftToRight":  1,
		"rightToLeft":  0,
		"rightToRight": 1,
	},
}

func GetEqListBassboostEarrape() []EqBand {
	return []EqBand{
		{Band: 0, Gain: 0.225},
		{Band: 1, Gain: 0.25125},
		{Band: 2, Gain: 0.25125},
		{Band: 3, Gain: 0.15},
		{Band: 4, Gain: -0.1875},
		{Band: 5, Gain: 0.05625},
		{Band: 6, Gain: -0.16875},
		{Band: 7, Gain: 0.08625},
		{Band: 8, Gain: 0.13125},
		{Band: 9, Gain: 0.16875},
	}
}
