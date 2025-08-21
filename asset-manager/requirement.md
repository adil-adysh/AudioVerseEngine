# AudioVerseEngine – AssetManager Requirements

## **Overview**

The AssetManager is a core subsystem of **AudioVerseEngine** responsible for loading, managing, and streaming game assets efficiently. It must support both **pre-decoded SFX** and **streamed music/dialogue** while being real-time safe and compatible with the engine’s ECS and Resonance Audio pipeline.

---

## **Functional Requirements**

1. **Asset Loading**

   * Load assets from `asset.pkg` or similar bundled formats.
   * Support multiple asset types:

     * `SFX` – small pre-decoded sounds (PCM f32).
     * `Stream` – large compressed audio (MP3, OGG, WAV) decoded in real-time.
   * Provide a consistent API to request assets by **name or ID**.

2. **SFX Handling**

   * Fully load small sound effects into memory.
   * Provide **interleaved f32 PCM** output.
   * Support optional metadata:

     * Channels
     * Sample rate
     * Loop points

3. **Streaming Music / Dialogue**

   * Decode audio on a **background thread**.
   * Feed a **lock-free ring buffer** for real-time playback.
   * Provide interleaved stereo PCM output.
   * Support streaming from both **pre-packaged assets** and **external files**.

4. **Asset Lifecycle Management**

   * Implement caching for frequently used assets.
   * Support optional eviction policies (LRU or size-based).
   * Handle asset unloading and cleanup safely.

5. **Error Handling**

   * Gracefully handle missing or corrupted assets.
   * Provide fallback asset (e.g., silence or default SFX).
   * Avoid crashing the audio thread.

---

## **Technical Requirements**

1. **Rust Core Engine Integration**

   * Expose safe, ergonomic Rust API for ECS systems and DSP components.
   * Compatible with `resonance-cxx` bridge for 3D audio playback.

2. **Threading & Real-Time Safety**

   * Streaming assets must decode in a **background thread**.
   * Use a **lock-free ring buffer** (`ringbuf` crate or equivalent) to supply PCM samples to the audio thread.

3. **Supported Formats**

   * Pre-decoded SFX: `.pcm` or `.sfx` binary blobs.
   * Streamed audio: `.ogg`, `.mp3`, `.wav` (via `symphonia`).

4. **Bundled Asset Support**

   * Read assets from `asset.pkg`:

     * Header with asset index
     * Metadata (type, offset, size, sample rate, channels)
     * Binary asset data (SFX or streamed)

5. **Cross-Platform Compatibility**

   * Windows (WASAPI / DirectSound)
   * Android (AAudio via JNI)

6. **Dependencies**

   * `bincode` for serialization
   * `serde` for metadata
   * `symphonia` for runtime decoding
   * `ringbuf` for streaming buffers

---

## **API Requirements**

* `load_sfx(name: &str) -> Result<Vec<f32>, Error>`
* `load_stream(name: &str) -> Result<StreamingAsset, Error>`
* `StreamingAsset`:

  * Producer (background thread writes PCM)
  * Consumer (audio thread reads PCM)
* Optional:

  * `unload(name: &str)`
  * `prefetch(name: &str)` for caching

---

## **Non-Functional Requirements**

* Minimal runtime overhead (especially for streaming).
* Deterministic behavior for real-time audio playback.
* Scalable for **hundreds of SFX** and multiple streaming tracks simultaneously.
* Robust error handling and logging.
