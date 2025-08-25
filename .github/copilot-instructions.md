## Purpose

Concise, practical instructions for AI coding agents and contributors working on AudioVerseEngine after the migration to Bevy.

Keep guidance lightweight and focused on `engine-core` (audio, ECS systems), `asset-manager` (audio formats), and application crates (visualization/debug). Prefer RON for editable world descriptors and keep rendering optional.

## What this repo is (1 line)

Audio-first game engine: Rust core + C++ Resonance Audio bridge + optional Flutter/Dart UI.



## Assets & tooling (practical notes)

- Packager: `tools/asset-packer` writes `assets/dest/out.pkg` using a 512-byte header placeholder and a `bincode` index. See `tools/asset-packer/src/main.rs`.
- Converter: use `tools/asset-utils::convert_to_sfx_bytes` to convert .wav/.ogg/.opus into canonical `.sfx` bytes (target 48 kHz, interleaved f32).
- Assets layout used by `--pack-assets`:
  - `assets/sfx` — sfx sources (.wav/.ogg/.opus) or `.sfx` files.
  - `assets/audio` — music/raw audio files.
  - `assets/dest` — outputs (e.g., `out.pkg`).

## .sfx format (must not change silently)

- Header: `"SFX1"` (4 bytes), sample format (u8: 0=F32), channels (u8), reserved 2 bytes, sample_rate (u32 LE), frames (u64 LE), then samples interleaved (f32 when format=0).
- Canonical parser: `asset-manager/src/sfx.rs::SfxBlob::from_sfx_bytes` and loader in `asset-manager/src/sfx_loader.rs`. Target sample rate: `TARGET_SAMPLE_RATE = 48000`.

## Resonance Audio integration (where to look)

- Public API: `resonance-audio/resonance_audio/api/resonance_audio_api.h` — use this as the engine external surface.
- Impl & patterns: `resonance-audio/resonance_audio/graph/resonance_audio_api_impl.h` (task queue, GraphManager), `resonance-audio/resonance_audio/graph/graph_manager.h` (source lifecycle), `resonance-audio/resonance_audio/utils/lockless_task_queue.h` (post setters to audio thread).
- Geometry & rooms: `geometrical_acoustics/scene_manager.h` and `platforms/common/room_properties.h` for room mapping.

## Where to change code safely (short guidance)

- Implement engine features in `engine-core/src/` and expose minimal FFI in `resonance-cxx/` if needed.
- For assets, change parser/loader in `asset-manager/src/` and adapt callers (`tools/asset-packer`).
- Add CLI/tools under `tools/` and include them in the workspace `Cargo.toml`.

## Quick test suggestion

- Add a unit test in `asset-manager/tests` that calls `tools::asset-utils::convert_to_sfx_bytes` (or a small internal helper) and asserts `SfxBlob::from_sfx_bytes` succeeds.

## Project layout & important docs

- Top-level crates/folders you will edit:
  - `engine-core/` — Rust core engine (ECS, DSP, source registry)
  - `resonance-cxx/` — Rust <-> C++ bridge (cxx) and C++ glue
  - `resonance-audio/` — bundled C++ resonance audio sources and headers
  - `asset-manager/`, `tools/asset-packer`, `tools/asset-utils` — asset formats and tooling
  - `audio-backend/` — OS audio backend implementations (Windows, mock)

- Key docs (read these first):
  - `docs/resonance_audio_crates_structure.md` — mapping of C++ headers to recommended Rust crates and crate boundaries.
  - `docs/audio-system-design.md` — data flow, threading model, asset manager and render-thread responsibilities.
  - `docs/cxx-reference.md` — bridge patterns, `#[cxx::bridge]` guidance, and build hints for `resonance-cxx`.
  - `docs/engine_headers_reference.md` — quick reference for important resonance-audio headers and the recommended Engine wrapper.
    - `docs/resonance_cxx_guidance.md` — step-by-step pattern and checklist for adding cxx bridge wrappers (new guidance).

Use these docs to understand where to place new code and how to map C++ APIs into Rust crates.

---

## (Bevy migration) Short guidance

Note: this repo migrated to use Bevy for ECS, asset management, and time. Keep `engine-core` renderer-agnostic and use RON world descriptors under `assets/worlds/` for audio-driven spatial data. Add rendering and `bevy_voxel_world` only in application crates when needed.
