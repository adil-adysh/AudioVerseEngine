Perfect! Let’s design a **next-gen Renderer** for your audio engine that’s real-time safe, supports multiple SFX and streams, spatialization, and introduces **buses/mixers**. I’ll break it down clearly.

---

## **1. Core Concepts**

**1. Voices (short SFX)**

* Pool of short sounds.
* Each has: `Arc<Vec<f32>>` buffer, playhead, gain, optional spatial source.

**2. Streams (background music / long audio)**

* Each stream has its own `HeapCons<f32>` ring buffer.
* Can be spatialized individually.

**3. Buses / Mixer**

* Each bus accumulates multiple voices or streams.
* Can apply:

  * Bus-level gain
  * Bus-level spatial effects (via Resonance source)
  * Optional limiting/clipping

**4. Command Queue**

* Still lock-free (`ArrayQueue<Command>`).
* Commands now support **bus assignment** and **bus-level parameters**.

**5. Scratch Buffers**

* Each bus gets a preallocated scratch buffer for mixing voices/streams.
* Avoids overwriting when multiple streams mix.

---

## **2. Suggested Data Structures**

```rust
pub struct Bus {
    scratch: Vec<f32>,          // preallocated buffer per bus
    gain: f32,                  // overall bus gain
    spatial_src_id: Option<i32>,// optional Resonance source for bus-level spatialization
}

pub struct Renderer {
    api: Api,
    num_channels: usize,
    frames_per_buffer: usize,

    voices: Vec<Voice>,
    streams: Vec<StreamSlot>,
    buses: Vec<Bus>,            // new

    sources: Vec<Option<i32>>,  // per-voice/stream source mapping

    cmd_queue: Arc<ArrayQueue<Command>>,
}
```

---

## **3. Mixer Flow**

**Per-frame processing:**

1. **Drain commands** (update voices, streams, buses).
2. **Zero buffers** (both main output and bus scratch buffers).
3. **Mix voices into assigned bus scratch buffer**

   * Apply per-voice gain.
   * Apply spatialization if `spatial_src_id` exists.
4. **Mix streams into assigned bus scratch buffer**

   * Pop from ring buffer.
   * Apply spatialization if needed.
5. **Mix all buses into main output buffer**

   * Apply bus gain.
   * Optionally: soft clip or limiter per bus.
6. **Send final buffer to `resonance_cxx::Api.fill_interleaved_f32`**

---

## **4. Bus / Spatialization Benefits**

* Allows **grouping multiple SFX** under one bus (e.g., explosions).
* **Bus-level spatialization**: you could have a single Resonance source representing all voices in a room.
* **Volume / effect control** at bus-level, rather than updating each voice individually.

---

## **5. Command Extensions**

```rust
pub enum Command {
    PlaySfx {
        slot: usize,
        buffer: SfxBuffer,
        gain: f32,
        pos: Option<Vec3>,
        bus: Option<usize>, // optional bus assignment
    },
    StartStream {
        slot: usize,
        ring: HeapCons<f32>,
        channels: usize,
        bus: Option<usize>,
    },
    SetBusGain { bus: usize, gain: f32 },
    SetBusSpatial { bus: usize, position: Vec3, rotation: Quat },
    // other existing commands...
}
```

---

## **6. Advantages**

1. **Scalable** – adding more voices/streams won’t break spatialization.
2. **Flexible** – buses allow for music, SFX, ambient sounds to be mixed separately.
3. **Real-Time Safe** – preallocated scratch buffers + lock-free queues.
4. **Optimized** – only voices/streams that are active are processed.

---

If you want, I can **draft the full Rust implementation of this next-gen Renderer** using your current `Voice` / `StreamSlot` design, including buses, spatialization integration, and safe mixing.

Do you want me to do that next?

