# Project TODOs — AudioVerseEngine

This file is a concise, prioritized set of actionable implementation tasks derived from `docs/audio-game-engine.md`.

Purpose

- Capture what the design doc defines (what's "designed") vs what still needs implementation.
- Provide prioritized, testable tasks with brief acceptance criteria and dependencies.

Quick summary of what the design doc contains

- Complete conceptual design for an audio-first game engine: modules for Core/ECS, Spatial, Physics, Audio, Events, Resources, Rooms, Runtime/GameLoop, Player/input, Prefab system, and Conceptual Architecture.
- Interfaces and data shapes described for Entities, Components, Systems, Scene, Transform, PhysicsComponent, AudioSource/Listener, EventBus, Asset types, Room/AcousticProfile, Prefab/ComponentDefinition, Time/GameLoop, and more.

What is implemented (code)

- Asset tooling and SFX handling:
  - `tools/sfx-convert/` CLI and `tools/asset-utils/src/lib.rs::convert_to_sfx_bytes` convert audio files to the project's `.sfx` format and resample to the canonical 48 kHz.
  - `asset-manager/src/sfx_loader.rs` implements `load_sfx_path_with_target`, `.sfx` parsing, resampling and `SfxMetadata`.
  - `tools/asset-packer/` packs assets into `assets/dest/out.pkg` and integrates with the sfx loader.
- Resonance audio bridge and runtime glue:
  - `resonance-cxx/` contains a `#[cxx::bridge]` wrapper (`src/bridge.rs`) and a safe Rust wrapper (`src/lib.rs`) that exposes `ResonanceAudioApi` (create/destroy sources, set buffers, head position, reverb/reflection properties) and convenience helpers.
  - Tests under `resonance-cxx/tests/` exercise the bridge and API surface.
- Resonance audio engine pieces:
  - `resonance-audio-engine/` crate contains `renderer`, `spatializer`, and `types` modules and tests that integrate with renderer and SFX types.
- Build and setup helpers:
  - `setup_resonance_audio.ps1` configures and builds the upstream C++ resonance-audio library for local development.
- Evidence & tests:
  - Integration tests reference the SFX loader and resonance bridge (examples: `resonance-cxx/tests/*`, `resonance-audio-engine/tests/*`).

Notes: the repo contains substantial native and tool support for audio assets and the Resonance Audio bridge; however the higher-level engine runtime pieces (ECS runtime, GameLoop, EventBus, PhysicsSystem, PrefabInstantiator) are still design-level and not present as a unified runtime in the codebase.

What needs implementing (high-level)

- ECS runtime (Entity store, Component storage, System registration, queries)
- GameLoop / Time management (frame loop, deltaTime computation)
- EventBus (publish/subscribe, queueing semantics, thread-safety)
- Physics system (broadphase/narrowphase minimal, collision events, material mapping)
- Audio system integration (asset loading, playback, spatialization, room acoustics hooks)
- Resource & Asset manager (audio assets, scene/prefab loading, caching)
- Environment & Room spatial queries and acoustic profiles
- Prefab system & PrefabInstantiator (parent inheritance, overrides)
- Player input component & input processing system
- Tests, examples and small demo scenes to validate audio feedback loops

Prioritized TODO list (actionable items)

1) Minimal ECS runtime (priority: High, effort: 3d)

- Goal: Provide a small, well-tested ECS used by engine subsystems.
- Tasks:
  - Implement Entity id allocation and storage.
  - Implement component storage (sparse map or HashMap per component type).
  - Implement basic queries: get entities with component set {A,B}.
  - Provide System trait with update(scene, Time).
- Acceptance: unit tests for entity creation, add/remove component, and a sample system that moves entities using a TransformComponent.
- Dependencies: none.

2) GameLoop & Time (priority: High, effort: 1d)

- Goal: Deterministic frame loop to call systems with deltaTime.
- Tasks:
  - Implement Time object with deltaTime and currentTime.
  - Implement a simple GameLoop runner that advances Time and calls systems in order.
- Acceptance: integration test that runs the loop for N frames and asserts systems executed expected number of times.
- Dependencies: ECS runtime.

3) EventBus (priority: High, effort: 1d)

- Goal: decoupled publish/subscribe for CollisionEvents, GameplayEvents, asset-loaded notifications.
- Tasks:
  - Implement publish/subscribe API with typed events or simple string-type + payload.
  - Ensure events published during a frame are processed before next frame (flush at end of update).
  - Add tests for subscriber invocation order and event payload passing.
- Acceptance: subscriber receives CollisionEvent published by PhysicsSystem in same frame.
- Dependencies: ECS, GameLoop.

4) Resource & Asset manager (priority: High, effort: 2d)

- Goal: load and cache audio assets and scene/prefab definitions.
- Tasks:
  - Implement `AudioAsset` loader that returns canonical `.sfx` / raw audio buffers.
  - Provide API to load `SceneDefinition` and `Prefab` JSON or TOML.
  - Add basic caching and path-resolution rules.
- Acceptance: unit test loads a sample scene JSON and audio asset and returns usable types for downstream systems.
- Dependencies: none (can be worked in parallel).

5) Prefab system & instantiator (priority: Medium, effort: 1-2d)

- Goal: resolve parent prefabs, apply overrides, produce Entity instances with components.
- Tasks:
  - Implement `Prefab` data model parser.
  - Implement `PrefabInstantiator` that takes prefab, creates Entity, applies nested parents and overrides.
  - Tests: inheritance and override behavior.
- Acceptance: prefab with parent and overrides instantiates into entity with expected component values.
- Dependencies: Resource manager, ECS.

Checkpoint — pause here and add a small demo scene that ties ECS, GameLoop, EventBus, and Resource manager together. (Recommended after items 1–5 complete)

6) Physics system (priority: Medium-High, effort: 3d)

- Goal: Minimal physics to generate `CollisionEvent`s (enough for audio feedback).
- Tasks:
  - Implement simple AABB collision detection or sphere-sphere checks for moving entities.
  - Material lookup per `PhysicsComponent` and contact point generation.
  - Publish CollisionEvent to EventBus.
  - Tests: collisions between two moving entities generate event with correct entities and contact point.
- Acceptance: integration test where two entities collide and AudioSystem receives event.
- Dependencies: ECS, EventBus, GameLoop.

7) Audio System (priority: High for audio features, effort: 3d)

- Goal: Play spatialized sounds in response to events using loaded assets and room acoustics.
- Tasks:
  - Integrate existing `resonance-cxx` bridge if present; otherwise implement a minimal mock backend for tests.
  - Implement AudioSource and AudioListener handling: map Transform to spatialization parameters.
  - Subscribe to CollisionEvent to play appropriate SFX using material mapping.
  - Room reverb/absorption application based on Environment & Rooms Module.
  - Tests: mocked playback driver verifies play calls with correct parameters (position, volume, asset id).
- Acceptance: unit/integration test where a collision triggers a playback call with expected asset id and position.
- Dependencies: Resource manager, EventBus, Spatial components, optional cxx bridge.

8) Environment & Rooms (priority: Medium, effort: 1-2d)

- Goal: Implement rooms with bounding volumes and acoustic profiles.
- Tasks:
  - Implement bounding-volume types (AABB, sphere) and point-in-volume checks.
  - Implement room lookup API and exposure to AudioSystem.
  - Tests: listener in room A uses room A's acoustic profile.
- Acceptance: AudioSystem applies room profile when listener position is inside room bounds.
- Dependencies: Spatial, Audio System.

9) Player & Input system (priority: Medium, effort: 1-2d)

- Goal: InputComponent and a system to update player Transform and publish GameplayEvents.
- Tasks:
  - Implement InputComponent and input mapping for simple actions (move, interact).
  - Provide a test harness to simulate input actions and assert Transform updates and events published.
- Acceptance: simulated input moves player entity and publishes appropriate events.
- Dependencies: ECS, GameLoop, EventBus.

10) Small demo scene + CI checks (priority: High, effort: 2d)

- Goal: End-to-end smoke test showing physics-driven audio: moving object collides → CollisionEvent → Audio playback.
- Tasks:
  - Create a small scene asset (JSON) with two entities, one audio listener, and one audio source mapping for collision.
  - Add unit/integration tests and a CI job (or GitHub Actions workflow) that builds and runs smoke tests.
  - Add a small placeholder C++ implementation for the resonance bridge if necessary for CI linking.
- Acceptance: CI job runs and passes smoke test.
- Dependencies: items 1–7.

Further improvements / stretch goals

- Fine-grained audio DSP: occlusion, late reflections, per-material convolution impulse responses.
- Editor tooling to author prefabs and acoustic profiles.
- Performance optimizations: spatial partitioning for large numbers of audio sources, lockless task queues for audio thread communication.

Estimates & priority notes

- Prioritize getting a minimal, testable audio feedback loop working: items 1,2,3,4,7,6,10.
- Work that can be parallelized: Resource manager, Prefab parsing, and basic ECS storage internals.

Quality gates

- Build: project should compile after implementing core crates; prefer small incremental PRs.
- Tests: provide unit tests for ECS, Prefab instantiation, EventBus, and AudioSystem behavior. Add one integration smoke test (demo scene).
- Lint/Format: run `cargo fmt` and `cargo clippy` on changed crates.

Requirements coverage

- Requirement: "rewrite the todos.md in docs" — Done: this file.
- Requirement: "understand what's already implemented" — Done: design-only documentation summarized.
- Requirement: "what needs to be implemented" — Done: explicit list above.
- Requirement: "create a list of todo" — Done.

Next steps I recommend

1. Review and adjust priorities based on available team resources and immediate goals (editor, runtime, or audio-first demo).
2. Start implementation with a tiny crate for ECS and tests, open small PRs iteratively.
3. For rapid validation, add a mocked audio backend so audio tests don't depend on native libraries.
