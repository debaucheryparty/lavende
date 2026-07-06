# Python Digital Signal Processing (DSP)

The `FilterManager` exposes Lavende's native DSP engine. Because the audio is manipulated directly in Rust, you can apply multiple overlapping filters simultaneously without suffering the severe CPU penalties normally associated with Python audio manipulation.

---

## Accessing the Filter Manager

Every `Player` instance possesses a `filter_manager` attribute.

```python
fm = player.filter_manager
```

---

## Available Filters

### 1. 15-Band Equalizer

The equalizer allows you to boost or attenuate specific frequency bands. Lavende supports 15 standard bands (0 through 14).

| Parameter | Type    | Range            | Description                       |
| :-------- | :------ | :--------------- | :-------------------------------- |
| `band`    | `int`   | `0` to `14`      | The frequency band to target.     |
| `gain`    | `float` | `-0.25` to `1.0` | The multiplier for the frequency. |

**Example: Applying a Bassboost**

```python
player.filter_manager.equalizer_bands = [
    {"band": 0, "gain": 0.25},
    {"band": 1, "gain": 0.20},
    {"band": 2, "gain": 0.10}
]
# Changes must be explicitly applied
await player.filter_manager.apply_player_filters()
```

### 2. Time-Stretching (Nightcore / Vaporwave)

You can independently manipulate the speed and pitch of the audio stream.

| Method             | Argument | Range           | Description                             |
| :----------------- | :------- | :-------------- | :-------------------------------------- |
| `set_speed(float)` | `speed`  | `0.01` - `10.0` | > 1.0 speeds up; < 1.0 slows down.      |
| `set_pitch(float)` | `pitch`  | `0.01` - `10.0` | > 1.0 raises pitch; < 1.0 lowers pitch. |

**Example: Nightcore**

```python
await player.filter_manager.set_speed(1.18)
await player.filter_manager.set_pitch(1.30)
```

**Example: Vaporwave**

```python
await player.filter_manager.set_speed(0.85)
await player.filter_manager.set_pitch(0.80)
```

### 3. Spatial 3D Audio

The rotation filter applies a panning effect to simulate the audio moving around the listener's head.

| Method                   | Argument | Description                         |
| :----------------------- | :------- | :---------------------------------- |
| `toggle_rotation(float)` | `hz`     | The speed of the rotation in Hertz. |

```python
# Rotate the audio at 0.3 Hz
await player.filter_manager.toggle_rotation(0.3)
```

### 4. Tremolo & Vibrato

Apply oscillating amplitude (tremolo) or pitch (vibrato) modulation effects.

```python
# Tremolo: oscillating amplitude
await player.filter_manager.toggle_tremolo(4.0, 0.8)

# Vibrato: oscillating pitch
await player.filter_manager.toggle_vibrato(10.0, 1.0)
```

### 5. Rate Control

Control the playback rate independently.

```python
await player.filter_manager.set_rate(1.0)
```

### 6. Volume Filter

Apply volume multiplication (different from player volume).

```python
await player.filter_manager.set_volume(1.5)  # 150% volume
```

### 7. Channel Forcing

Force the output into a specific channel configuration.

```python
await player.filter_manager.set_audio_output('mono')
await player.filter_manager.set_audio_output('stereo')
await player.filter_manager.set_audio_output('left')
await player.filter_manager.set_audio_output('right')
```

---

## Resetting State

To strip all active filters and return the stream to its original, unmodified state instantly:

```python
await player.filter_manager.reset_filters()
```

> [!WARNING]
> Depending on network latency and Discord's internal buffer, it may take between 100ms to 500ms for a filter change to reflect audibly to users in the voice channel.
