# Rust Digital Signal Processing (DSP)

The `FilterManager` provides access to Lavende's native DSP pipeline written in high-performance Rust. Apply real-time audio effects without external dependencies like ffmpeg.

---

## Accessing the Filter Manager

Every `LavendePlayer` contains a `FilterManager` protected by a `RwLock`.

```rust
{
    let mut filter_mgr = player.filter_manager.write().await;
    // Apply filters here
}

// Don't forget to apply!
player.apply_filters().await;
```

---

## Available Filters

### 1. 15-Band Equalizer

Boost or cut specific frequency bands ranging from `0` to `14`.

| Parameter | Type  | Range            | Description                       |
| :-------- | :---- | :--------------- | :-------------------------------- |
| `band`    | `u8`  | `0` to `14`      | The specific frequency band.      |
| `gain`    | `f64` | `-0.25` to `1.0` | The multiplier for the frequency. |

**Example: Applying Bassboost**

```rust
use lavende::EqBand;

{
    let mut filter_mgr = player.filter_manager.write().await;

    let bass_boost = vec![
        EqBand { band: 0, gain: 0.25 },
        EqBand { band: 1, gain: 0.30 },
        EqBand { band: 2, gain: 0.20 },
        EqBand { band: 3, gain: 0.10 },
        EqBand { band: 4, gain: 0.05 },
    ];

    filter_mgr.set_equalizer(bass_boost);
}

player.apply_filters().await;
```

### 2. Time-Stretching (Nightcore / Vaporwave)

Independently manipulate speed and pitch of the audio stream.

| Method           | Argument | Range           | Description                             |
| :--------------- | :------- | :-------------- | :-------------------------------------- |
| `set_speed(f64)` | `speed`  | `0.01` - `10.0` | > 1.0 speeds up; < 1.0 slows down.      |
| `set_pitch(f64)` | `pitch`  | `0.01` - `10.0` | > 1.0 raises pitch; < 1.0 lowers pitch. |
| `set_rate(f64)`  | `rate`   | `0.01` - `10.0` | Playback rate modification.             |

**Example: Nightcore Effect**

```rust
{
    let mut filter_mgr = player.filter_manager.write().await;
    filter_mgr.set_speed(1.18);
    filter_mgr.set_pitch(1.30);
}
player.apply_filters().await;
```

**Example: Vaporwave Effect**

```rust
{
    let mut filter_mgr = player.filter_manager.write().await;
    filter_mgr.set_speed(0.85);
    filter_mgr.set_pitch(0.80);
}
player.apply_filters().await;
```

**Combined Timescale**

```rust
{
    let mut filter_mgr = player.filter_manager.write().await;
    filter_mgr.set_timescale(1.25, 1.1, 1.0);
}
player.apply_filters().await;
```

### 3. Spatial 3D Audio

The rotation filter applies an oscillating panning effect to simulate audio orbiting around the listener.

| Method                 | Argument | Description                  |
| :--------------------- | :------- | :--------------------------- |
| `toggle_rotation(f64)` | `hz`     | The rotation speed in Hertz. |

```rust
{
    let mut filter_mgr = player.filter_manager.write().await;
    // Rotate at 0.2 Hz
    filter_mgr.toggle_rotation(0.2);
}
player.apply_filters().await;
```

### 4. Tremolo & Vibrato

Apply oscillating amplitude (tremolo) or pitch (vibrato) modulation.

```rust
{
    let mut filter_mgr = player.filter_manager.write().await;

    // Tremolo: amplitude modulation
    filter_mgr.toggle_tremolo(4.0, 0.8);

    // Vibrato: pitch modulation
    filter_mgr.toggle_vibrato(10.0, 1.0);
}
player.apply_filters().await;
```

### 5. Channel Configuration

Force audio output to specific channel configurations.

```rust
{
    let mut filter_mgr = player.filter_manager.write().await;

    // Mono output
    filter_mgr.set_audio_output("mono");

    // Stereo output (default)
    filter_mgr.set_audio_output("stereo");

    // Left channel only
    filter_mgr.set_audio_output("left");

    // Right channel only
    filter_mgr.set_audio_output("right");
}
player.apply_filters().await;
```

### 6. Volume Filter

Apply volume multiplication (different from player volume).

```rust
{
    let mut filter_mgr = player.filter_manager.write().await;

    // 150% volume
    filter_mgr.set_volume(1.5);
}
player.apply_filters().await;
```

---

## Applying Filters

> [!IMPORTANT]
> After modifying filters, you **must** call `apply_filters()` to sync changes with the audio engine.

```rust
{
    let mut filter_mgr = player.filter_manager.write().await;
    filter_mgr.set_speed(1.5);
    filter_mgr.set_pitch(1.2);
}

// Apply all pending filter changes
player.apply_filters().await;
```

---

## Resetting Filters

Remove all active filters and return to the original audio stream.

```rust
{
    let mut filter_mgr = player.filter_manager.write().await;
    filter_mgr.reset_filters();
}
player.apply_filters().await;
```

---

## Filter Serialization

Get the current filter state as JSON.

```rust
let filter_json = player.filter_manager.read().await.to_json();
println!("Current filters: {}", filter_json);
```

Or apply filters directly from JSON:

```rust
let filter_json = r#"{"volume": 1.5, "timescale": {"speed": 1.25}}"#;
player.set_filters(filter_json.to_string()).await;
```

> [!WARNING]
> DSP operations occur on a buffered audio thread. Filter changes may take a fraction of a second to become audible over the Discord voice stream.
