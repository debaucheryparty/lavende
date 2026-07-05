# Node.js Digital Signal Processing (DSP)

The `FilterManager` gives you raw access to Lavende's native DSP pipeline. Because the audio is mutated directly in Rust before RTP packaging, you can apply complex filters without the high CPU cycles usually demanded by Node.js or `ffmpeg`.

---

## Accessing the Filter Manager

Every `Player` instance manages its own `FilterManager`.

```javascript
const fm = player.filterManager;
```

---

## Available Filters

### 1. 15-Band Equalizer

The equalizer lets you boost or cut specific frequency bands (ranging from `0` to `14`).

| Parameter | Type | Range | Description |
| :--- | :--- | :--- | :--- |
| `band` | `Number` | `0` to `14` | The specific frequency band. |
| `gain` | `Number` | `-0.25` to `1.0` | The multiplier for the frequency. |

**Example: Applying a Bassboost**
```javascript
player.filterManager.equalizerBands = [
    { band: 0, gain: 0.25 },
    { band: 1, gain: 0.20 },
    { band: 2, gain: 0.10 }
];
// Must be called explicitly to flush changes to Rust
await player.filterManager.applyPlayerFilters();
```

### 2. Time-Stretching (Nightcore / Vaporwave)

You can independently manipulate the speed and pitch of the audio stream.

| Method | Argument | Range | Description |
| :--- | :--- | :--- | :--- |
| `setSpeed(Number)` | `speed` | `0.01` - `10.0` | > 1.0 speeds up; < 1.0 slows down. |
| `setPitch(Number)` | `pitch` | `0.01` - `10.0` | > 1.0 raises pitch; < 1.0 lowers pitch. |

**Example: Nightcore**
```javascript
await player.filterManager.setSpeed(1.18);
await player.filterManager.setPitch(1.30);
```

**Example: Vaporwave**
```javascript
await player.filterManager.setSpeed(0.85);
await player.filterManager.setPitch(0.80);
```

### 3. Spatial 3D Audio

The rotation filter applies an oscillating panning effect to simulate the audio orbiting the listener's head.

| Method | Argument | Description |
| :--- | :--- | :--- |
| `toggleRotation(Number)` | `hz` | The speed of the rotation in Hertz. |

```javascript
// Rotate the audio at 0.3 Hz
await player.filterManager.toggleRotation(0.3);
```

### 4. Channel Forcing

Force the output into a specific channel configuration.

```javascript
await player.filterManager.setAudioOutput('mono');
await player.filterManager.setAudioOutput('stereo');
```

---

## Resetting State

To strip all active filters and return the stream to its original, unmodified state instantly:

```javascript
await player.filterManager.resetFilters();
```

> [!WARNING]
> Depending on network latency and Discord's internal buffer, it may take between 100ms to 500ms for a filter change to audibly reflect to users in the voice channel.
