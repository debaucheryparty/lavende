# Golang Digital Signal Processing (DSP)

The `FilterManager` is the Go gateway to Lavende's native DSP pipeline. Because audio manipulation is written in high-performance Rust, you can apply extreme time-stretching and equalizers without the crippling CPU footprint of `ffmpeg`.

---

## Accessing the Filter Manager

Every `Player` struct contains a `FilterManager` instance.

```go
fm := player.FilterManager
```

---

## Available Filters

### 1. 15-Band Equalizer

The equalizer allows you to boost or cut specific frequency bands (ranging from `0` to `14`).

| Parameter | Type | Range | Description |
| :--- | :--- | :--- | :--- |
| `Band` | `int` | `0` to `14` | The specific frequency band. |
| `Gain` | `float64` | `-0.25` to `1.0` | The multiplier for the frequency. |

**Example: Applying a Bassboost**
```go
player.FilterManager.EqualizerBands = []lavende.EqBand{
    {Band: 0, Gain: 0.25},
    {Band: 1, Gain: 0.20},
    {Band: 2, Gain: 0.10},
}
// You must explicitly call Apply to sync changes with Rust
player.FilterManager.ApplyPlayerFilters()
```

### 2. Time-Stretching (Nightcore / Vaporwave)

You can independently manipulate the speed and pitch of the audio stream.

| Method | Argument | Range | Description |
| :--- | :--- | :--- | :--- |
| `SetSpeed(float64)` | `speed` | `0.01` - `10.0` | > 1.0 speeds up; < 1.0 slows down. |
| `SetPitch(float64)` | `pitch` | `0.01` - `10.0` | > 1.0 raises pitch; < 1.0 lowers pitch. |

**Example: Nightcore**
```go
player.FilterManager.SetSpeed(1.18)
player.FilterManager.SetPitch(1.30)
```

**Example: Vaporwave**
```go
player.FilterManager.SetSpeed(0.85)
player.FilterManager.SetPitch(0.80)
```

### 3. Spatial 3D Audio

The rotation filter applies an oscillating panning effect to simulate the audio orbiting the listener's head.

| Method | Argument | Description |
| :--- | :--- | :--- |
| `ToggleRotation(float64)` | `hz` | The speed of the rotation in Hertz. |

```go
// Rotate the audio at 0.3 Hz
player.FilterManager.ToggleRotation(0.3)
```

### 4. Tremolo & Vibrato

Apply oscillating amplitude (tremolo) or pitch (vibrato) modulation effects.

```go
// Tremolo: oscillating amplitude
player.FilterManager.ToggleTremolo(4.0, 0.8)

// Vibrato: oscillating pitch
player.FilterManager.ToggleVibrato(10.0, 1.0)
```

### 5. Rate Control

Control the playback rate independently.

```go
player.FilterManager.SetRate(1.0)
```

### 6. Volume Filter

Apply volume multiplication (different from player volume).

```go
player.FilterManager.SetVolume(1.5) // 150% volume
```

### 7. Channel Forcing

Force the output into a specific channel configuration.

```go
player.FilterManager.SetAudioOutput("mono")
player.FilterManager.SetAudioOutput("stereo")
player.FilterManager.SetAudioOutput("left")
player.FilterManager.SetAudioOutput("right")
```

---

## Resetting State

To strip all active filters and return the stream to its original, unmodified state instantly:

```go
player.FilterManager.ResetFilters()
```

> [!WARNING]
> Because DSP manipulation happens on a deeply buffered native thread, it may take a fraction of a second for filter changes to become audible over the Discord voice stream.
