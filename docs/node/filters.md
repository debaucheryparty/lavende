# Node.js Digital Signal Processing (DSP)

The `FilterManager` gives you raw access to Lavende's native DSP pipeline. Because the audio is mutated directly in Rust before RTP packaging, you can apply complex filters without the high CPU cycles usually demanded by Node.js or `ffmpeg`.

---

## Accessing the Filter Manager

Every `Player` instance manages its own `FilterManager`.

```typescript
// TypeScript & JavaScript
const fm = player.filterManager;
```

---

## Available Filters

### 1. 15-Band Equalizer

The equalizer lets you boost or cut specific frequency bands (ranging from `0` to `14`).

| Parameter | Type     | Range            | Description                       |
| :-------- | :------- | :--------------- | :-------------------------------- |
| `band`    | `number` | `0` to `14`      | The specific frequency band.      |
| `gain`    | `number` | `-0.25` to `1.0` | The multiplier for the frequency. |

**Example: Applying a Bassboost**

```typescript
player.filterManager.equalizerBands = [
  { band: 0, gain: 0.25 },
  { band: 1, gain: 0.2 },
  { band: 2, gain: 0.1 },
];
// Must be called explicitly to flush changes to Rust
await player.filterManager.applyPlayerFilters();
```

### 2. Time-Stretching (Nightcore / Vaporwave)

You can independently manipulate the speed and pitch of the audio stream.

| Method                    | Argument | Range           | Description                             |
| :------------------------ | :------- | :-------------- | :-------------------------------------- |
| `setSpeed(speed: number)` | `speed`  | `0.01` - `10.0` | > 1.0 speeds up; < 1.0 slows down.      |
| `setPitch(pitch: number)` | `pitch`  | `0.01` - `10.0` | > 1.0 raises pitch; < 1.0 lowers pitch. |

**Example: Nightcore**

```typescript
await player.filterManager.setSpeed(1.18);
await player.filterManager.setPitch(1.3);
```

**Example: Vaporwave**

```typescript
await player.filterManager.setSpeed(0.85);
await player.filterManager.setPitch(0.8);
```

### 3. Spatial 3D Audio

The rotation filter applies an oscillating panning effect to simulate the audio orbiting the listener's head.

| Method                       | Argument | Description                         |
| :--------------------------- | :------- | :---------------------------------- |
| `toggleRotation(hz: number)` | `hz`     | The speed of the rotation in Hertz. |

```typescript
// Rotate the audio at 0.3 Hz
await player.filterManager.toggleRotation(0.3);
```

### 4. Tremolo & Vibrato

Apply oscillating amplitude (tremolo) or pitch (vibrato) modulation effects.

```typescript
// Tremolo: oscillating amplitude
await player.filterManager.toggleTremolo(4.0, 0.8);

// Vibrato: oscillating pitch
await player.filterManager.toggleVibrato(10.0, 1.0);
```

### 5. Rate Control

Control the playback rate independently.

```typescript
await player.filterManager.setRate(1.0);
```

### 6. Volume Filter

Apply volume multiplication (different from player volume).

```typescript
await player.filterManager.setVolume(1.5); // 150% volume
```

### 7. Channel Forcing

Force the output into a specific channel configuration. Valid options are `'mono'`, `'stereo'`, `'left'`, or `'right'`.

```typescript
await player.filterManager.setAudioOutput("mono");
await player.filterManager.setAudioOutput("stereo");
await player.filterManager.setAudioOutput("left");
await player.filterManager.setAudioOutput("right");
```

---

## Resetting State

To strip all active filters and return the stream to its original, unmodified state instantly:

```typescript
await player.filterManager.resetFilters();
```

> [!WARNING]
> Depending on network latency and Discord's internal buffer, it may take between 100ms to 500ms for a filter change to audibly reflect to users in the voice channel.
