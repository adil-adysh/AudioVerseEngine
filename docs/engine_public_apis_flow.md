# Engine Public APIs – Flow & Logic (ECS + Audio Game Context)

Absolutely — using your detailed **Audio Game Design** context, this document rewrites the **Engine Public APIs Flow** so it naturally aligns with the ECS, systems, spatial, physics, audio, and event-driven architecture, while focusing on public API flow and logic. Internal types are kept hidden from public-facing signatures
---

The engine exposes high-level, safe APIs for entity management, audio playback, transforms, environmental effects, and runtime control. Internally, all public calls interact with the World, Systems, EventBus, Audio/Mixer Services, and Spatial/Physics modules, but developers see only intuitive operations.

This document explains how public APIs behave, how data flows, and how components interact, while respecting the ECS-based modular design.

---

## 1. Engine Initialization

Public API: `Engine::new(config)`

Flow:

1. Creates a `World` instance, which includes:

   - `EntityManager` for entity lifecycles.
   - `ComponentManager` for managing components.
   - `SystemManager` for registering and updating systems.
   - `EventBus` for event-driven communication.
   - `SpatialQueryService`, `DebugService`, `BootstrapScriptingService`.
2. Internal audio systems (`AudioSystem`, `AudioListenerSystem`) and `MixerService` are initialized but hidden from the public surface.
3. Developers now have a ready-to-use engine, with the world bootstrapped by the GameLoop.

Note: Actual system registration and one-time setup happen in `World.bootstrap()`, called by the GameLoop.

---

## 2. Game Loop Integration

Public API: `engine.update(deltaTime)`

Flow:

1. Fixed update loop runs deterministic systems:

   - `PhysicsTransformSystem`
   - `PhysicsSystem`
2. Variable update loop runs:

   - `InputSystem`
   - `AudioListenerSystem` → caches listener transform.
   - `RenderTransformSystem`
   - `AmbientZoneSystem`
   - `AudioSystem` → updates playback state based on queued commands.
   - `RenderingSystem`
3. All entity/component updates, audio processing, spatialization, and event handling are performed internally.
4. Developers only call `update` once per frame; the engine orchestrates the full pipeline.

---

## 3. Entity Lifecycle APIs

Public APIs:

- `create_entity()` → creates a new entity.
- `destroy_entity(entity)` → schedules entity and attached components for cleanup.

Flow:

1. `create_entity` returns a handle (`Entity`) that can receive components.
2. `destroy_entity` enqueues removal; actual cleanup occurs during the next `update` to prevent dangling references.
3. Systems (Physics, Audio, Transform) are notified via `SystemManager.entityDestroyed(entity)` internally.

---

## 4. Audio Component APIs

Public APIs:

- `add_sound(entity, assetId)` → attach audio source.
- `add_stream(entity, streamId)` → attach streaming audio source.

Flow:

1. Short clips are cached for low-latency playback; long streams are handled asynchronously.
2. Components wait for explicit playback commands; `AudioSystem` responds to `PlaySound` events.
3. Spatialization, priority, voice-stealing, and concurrency are computed automatically via internal systems.

---

## 5. Playback Control APIs

Public APIs:

- `play(entity)` → enqueue playback request.
- `pause(entity)` → enqueue pause request.
- `stop(entity)` → enqueue stop request.

Flow:

1. Each call pushes a strongly typed event (`PlaySoundEventPayload`, `StopSoundEventPayload`) to the `EventBus`.
2. `AudioSystem` consumes events on the next `update`:

   - Allocates mixer voice channels.
   - Updates `AudioPlaybackStateComponent`.
   - Applies spatialization using `AudioListenerComponent` and `WorldTransformComponent`.
   - Pushes commands to `MixerService` internally.

Thread-safe, deterministic, and fully ECS-compliant.

---

## 6. Transform & Listener APIs

Public APIs:

- `set_position(entity, Vec3)`
- `set_orientation(entity, Quat)`
- `set_listener(listenerEntity)`

Flow:

1. Updates stored in `TransformComponent`.
2. Systems like `RenderTransformSystem` and `PhysicsTransformSystem` compute world transforms.
3. `AudioListenerSystem` detects the listener entity, publishing `ListenerTransformEventPayload`.
4. `AudioSystem` automatically spatializes sounds relative to the listener.

Developers don’t handle low-level math or spatial audio directly.

---

## 7. Environmental & Effect APIs

Public APIs:

- `add_reverb_zone(entity, params)` → attach environmental zone.
- `add_filter(entity, params)` → attach audio filter.

Flow:

1. `AmbientZoneSystem` tracks listener zone membership.
2. Audio layers start/stop automatically via `MixerService` commands.
3. Effects are applied dynamically based on priority, daypart, and proximity.

Developers never manually apply effects per frame.

---

## 8. Resource & Asset APIs

Public APIs:

- `loadAsync(assetId)` → load asset asynchronously.
- `release(assetId)` → decrement reference.
- `openAudioStream(assetId)` → prepare streaming audio.
- `closeAudioStream(streamId)` → close stream.

Flow:

1. Commands are pushed to `ResourceLoader`’s queue.
2. On `update`, assets are loaded/released safely.
3. Audio streams are decoded into ring buffers for playback.
4. Streaming backend (memory-mapped, std I/O, or web fetch) operates automatically.

---

## 9. Shutdown & Cleanup

Public API: `engine.shutdown()`

Flow:

1. Enqueues shutdown event.
2. Stops audio playback and mixer service.
3. Cleans up all entities, components, and system states.
4. Frees resources and safely terminates background services.

---

## 10. End-to-End Example Flow

Scenario: Play an explosion sound in a spatial zone.

1. `entity = create_entity()` → new entity.
2. `add_sound(entity, "explosion.wav")` → attach audio.
3. `set_position(entity, {x:10, y:0, z:5})` → place in world.
4. `play(entity)` → enqueue playback.
5. `update(dt)` → engine handles:

   - Command queue dispatches `PlaySound` event.
   - `AudioSystem` retrieves source, checks spatial options and listener transform.
   - `MixerService` allocates voice and blends audio.
   - `AmbientZoneSystem` applies crossfade if inside a zone.
   - Final audio buffer output via renderer.

From developer perspective: one-line commands trigger a fully coordinated ECS + Audio + Mixer + Spatial pipeline.

---

## Summary

The public-facing engine API includes:

- Entity management: `create_entity`, `destroy_entity`
- Component attachment: `add_sound`, `add_stream`, `set_position`, `set_orientation`
- Playback control: `play`, `pause`, `stop`
- Listener & environment: `set_listener`, `add_reverb_zone`, `add_filter`
- Asset/resource management: `loadAsync`, `release`, `openAudioStream`, `closeAudioStream`
- Runtime: `update(deltaTime)`, `shutdown()`

All audio spatialization, mixer, physics, and ECS coordination are internal, hidden from developers, ensuring thread-safe, deterministic, and accessibility-friendly operation.

---

If you want, I can draw a visual flowchart of Public API → EventBus → Systems → Mixer → Renderer reflecting the ECS + Audio Game Design structure. This would make it extremely clear how calls propagate.

Do you want me to make that flowchart?
