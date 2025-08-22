
# AudioVerseEngine

AudioVerseEngine is an **audio-first game engine** designed to make it easy to build **accessible audio games** for **Windows and Android**. Unlike traditional engines such as Unity or Unreal, which are heavily visual, AudioVerseEngine allows developers—especially visually impaired creators—to build immersive audio experiences using modern, general-purpose languages like **Rust** and **Dart**.

The engine leverages the **Resonance Audio SDK** for spatial audio, supports **low-latency audio backends**, and provides a **Flutter/Dart layer** for cross-platform UI and packaging.

---

## Motivation

Existing audio game solutions present several challenges:

* **Unity / Unreal Engine**: Difficult for visually impaired developers due to visual editors.
* **Blastbay Game Toolkit (Windows-only)**: Limited to Windows and does not use modern general-purpose languages.
* **AudioVerseEngine Goal**: Provide a cross-platform, modern, and accessible **audio-first game engine** that can be developed without relying on a GUI-heavy workflow.

---

## Goals

* Expose a **performant Rust core engine** for audio source management, spatialization, and ECS-based audio logic.
* Provide **safe bindings to Resonance Audio** via a **C++/Rust cxx bridge** for binaural and room acoustics.
* Offer **platform-specific audio backends**:

  * Windows: WASAPI / DirectSound.
  * Android: AAudio via NDK + JNI for device discovery.
* Enable developers to build and package games using **Flutter/Dart**, allowing cross-platform deployment and UI logic in a familiar language.
* Prioritize **accessibility**, low-latency audio, and ease-of-use for visually impaired developers.

---

## Architecture Overview

AudioVerseEngine has **three main layers**: Rust Core Engine, C++ Resonance Audio bridge, and Dart/Flutter UI & game logic.

```
+-----------------------------+
|      Flutter / Dart UI      |
|  Game logic, input handling |
|  Cross-platform packaging   |
+-------------+---------------+
              |
              | FFI bridge
              v
+-----------------------------+
|       Rust Core Engine      |
| - ECS (Entities & Components)|
| - Source registry & routing |
| - Ring buffers & audio DSP  |
| - Listener & spatialization |
+-------------+---------------+
              |
              | cxx bridge / FFI
              v
+-----------------------------+
|      C++ Resonance Audio    |
| - Spatial audio processing  |
| - Room acoustics & HRTFs    |
| - Low-level audio backend   |
+-----------------------------+
              |
              v
+-----------------------------+
|      Platform Audio Backend |
| - Windows: WASAPI / DirectSound |
| - Android: NDK / AAudio + JNI |
+-----------------------------+
```

**Flow Explanation**:

1. **Flutter/Dart Layer**: Game logic and UI; interacts with Rust engine through **FFI**.
2. **Rust Core Engine**: Manages ECS-based audio sources, routing, and spatialization.
3. **C++ Resonance Audio Layer**: Handles high-performance 3D audio and room acoustics.
4. **Platform Audio Backend**: Interfaces with OS-level audio APIs for low-latency output.

---

## Current Status

* **Implemented**:

  * Rust core engine design (crates & architecture planned).
  * `resonance-cxx` crate (C++ bridge to Resonance Audio).
  * Windows audio backend with mock and tested implementation.

* **Pending / Not Yet Implemented**:

  * Full Rust engine core (ECS systems, source registry, ring buffers).
  * Android backend + Flutter/Dart integration.
  * Cross-platform build scripts, packaging, and example projects.

---

## High-Level Tasks (Contributor-Friendly)

1. Implement **Rust core engine**: ECS, sources, listener, routing, ring buffers.
2. Extend **audio backend** for Android (NDK / AAudio) with JNI device discovery.
3. Implement **Flutter/Dart integration** for game logic and UI.
4. Add **example audio games** and minimal templates.
5. Write **unit tests** for Rust core, DSP logic, and backend systems.
6. Implement **cross-platform packaging** for Windows and Android via Flutter.

---

## Setup Instructions (Windows)

1. Clone the repository with submodules:

```bash
git clone --recurse-submodules https://github.com/your-org/AudioVerseEngine.git
cd AudioVerseEngine
git submodule update --init --recursive
```

2. Install dependencies:

   * **MSVC 2022 Build Tools**
   * **Windows 10 SDK**
   * **CMake**

3. Configure the build environment (PowerShell example):

```powershell
.\scripts\setup-env.ps1
```

This sets up `INCLUDE`, `LIB`, and `PATH` for MSVC and SDK.

4. Build the engine:

```bash
cargo build
cargo test
```

## Assets & tooling

- The repository includes simple asset tooling for creating game asset packages. The packer and helper tools live under `tools/`:

  - `tools/asset-packer` — packs files into a binary `asset.pkg` with a 512-byte header placeholder and a `bincode`-serialized index.
  - `tools/asset-utils` — shared utilities for converting audio files (WAV/OGG/OPUS) into the project's `.sfx` blob format.

- Recommended quick workflow (PowerShell):

  ```powershell
  # from repo root
  cargo run -p asset-packer -- --pack-assets
  # or use wrapper if present
  .\tools\pack-assets.ps1
  ```

- Assets layout expected by `--pack-assets` mode:

  - `assets/sfx` — source audio files (.wav/.ogg/.opus) or prebuilt `.sfx` files (converted to `.sfx` by `asset-utils`).
  - `assets/audio` — raw music/audio files packed as `Music` assets.
  - `assets/dest` — destination for `out.pkg` created by the packer.

- `.sfx` format (canonical parser: `asset-manager/src/sfx.rs`): header `"SFX1"` (4 bytes), sample format byte (0=F32), channels (u8), reserved 2 bytes, sample_rate (u32 LE), frames (u64 LE), then interleaved samples (f32 for format 0). The project expects 48 kHz interleaved f32 in memory (see `asset-manager::sfx_loader::TARGET_SAMPLE_RATE`).


---

## Android Notes

* Android backend is **planned**: using **NDK/A-Audio** for low-latency audio.
* Device discovery will be handled via **JNI**.
* Flutter plugin will expose minimal audio game runtime API to Dart.

---

## Contribution Guidelines

* Prioritize **core engine implementation** and **cross-platform backends**.
* Use the **Rust crate structure** defined in `docs/resonance_audio_crates_structure.md`.
* Add **tests and CI** for all new functionality.
* Contributions that include **small, runnable examples** and **CI-friendly builds** are highly encouraged.
* For Android, contributions should include **AAudio + JNI integration** and example Flutter/Dart usage.

---

## Documentation

* `docs/` contains:

  * Rust & C++ crate structure
  * Threading and real-time audio guidelines
  * cxx bridge guidance
  * Task lists and design notes

---

## License

AudioVerseEngine is open-source under the **MIT License**.
