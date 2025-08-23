# audio  game design

1. Core World & Entities Module
1.1. Module Responsibility
This module forms the core of the engine's ECS. It manages all entities and components in a highly performant, data-oriented way, and provides a central World interface for the entire engine.

1.2. Type Definitions & Interfaces
Entity: A unique handle.

type Entity = number;

Signature: A bitmask representing an entity's component composition, implemented as a number for performance-critical bitwise operations.

type Signature = number;

Component: The base interface for all data-only components.

interface Component {
    // All components must be plain data structures
}

System: The base interface for all logic-based systems.

interface System {
    update(entities: Entity[], deltaTime: number): void;
    getSignature(): Signature; // Add a method to explicitly get the signature
    onEntityDestroyed(entity: Entity): void; // New: Notifies systems of a destroyed entity for cleanup.
}

World: The central hub for the engine, delegating responsibilities to its managers.

interface World {
    readonly entityManager: EntityManager;
    readonly componentManager: ComponentManager;
    readonly systemManager: SystemManager;
    readonly eventBus: EventBus;
    bootstrap(): void; // New: Dedicated method for a one-time world setup.
    readonly spatialQuery: SpatialQueryService;
    readonly debug: DebugService;
    readonly bootstrapScripting: BootstrapScriptingService;
}

The World needs a way to be initialized and to expose the main managers for external use.

class World {
    public readonly entityManager: EntityManager;
    public readonly componentManager: ComponentManager;
    public readonly systemManager: SystemManager;
    public readonly eventBus: EventBus;
    public readonly spatialQuery: SpatialQueryService;
    public readonly debug: DebugService;
    public readonly bootstrapScripting: BootstrapScriptingService;

    constructor() {
        // Constructor is now only for creation; no side effects.
        this.entityManager = new EntityManager();
        this.componentManager = new ComponentManager();
        this.eventBus = new EventBus();
        this.systemManager = new SystemManager(this.entityManager, this.componentManager, this.eventBus);
        this.spatialQuery = new SpatialQueryService();
        this.debug = new DebugService();
        this.bootstrapScripting = new BootstrapScriptingService();
    }

    /**
     * Bootstraps the world by registering all systems and setting up event handlers.
     * This method should be called only once by the GameLoop.
     */
    public bootstrap(): void {
        // Example system registration. This logic is now here, not in the constructor.
        // this.systemManager.registerFixedSystem(new PhysicsSystem(...), 1);
        // ...
    }
}

EntityManager: Manages entity lifecycles and component signatures.

interface EntityManager {
    createEntity(): Entity;
    destroyEntity(entity: Entity): void;
    getSignature(entity: Entity): Signature;
    setSignature(entity: Entity, signature: Signature): void; // Uses bitwise operations on the number type
}

ComponentManager: Manages all component pools.

interface ComponentManager {
    addComponent<T extends Component>(entity: Entity, component: T): void;
    getComponent<T extends Component>(entity: Entity): T | undefined;
    removeComponent<T extends Component>(entity: Entity): void;
    // getAllComponents method has been removed to enforce a data-oriented design.
}

SystemManager: Manages the registration and execution of all systems.

interface SystemManager {
    /**
     *Registers a system that should be run in the fixed update loop.
     * The order parameter ensures deterministic execution.
     */
    registerFixedSystem<T extends System>(system: T, order: number): void;
    /**
     *Registers a system that should be run in the variable update loop.
     *The order parameter ensures deterministic execution.
     */
    registerVariableSystem<T extends System>(system: T, order: number): void;
    runFixedUpdate(deltaTime: number): void;
    runVariableUpdate(deltaTime: number): void;
    entityDestroyed(entity: Entity): void;
    entitySignatureChanged(entity: Entity, signature: Signature): void;
    /**
     *Retrieves a registered system instance by its type.
     *Throws an error if the system is not found. Use this when the system is
     *a required dependency.
     */
    getRequiredSystem<T extends System>(systemType: new(...args: any[]) => T): T;
    /**
     *Retrieves a registered system instance by its type, or undefined if not found.
     *Use this when the system is an optional dependency.
     */
    getOptionalSystem<T extends System>(systemType: new(...args: any[]) => T): T | undefined;
}

2. Spatial Definition Module
2.1. Module Responsibility
This module explicitly manages an entity's spatial data, separating local and world-space transforms to prevent redundant calculations.

2.2. Type Definitions & Interfaces
Vec3 & Quat: Standard 3D vector and quaternion types.

type Vec3 = { x: number; y: number; z: number; };
type Quat = { x: number; y: number; z: number; w: number; };
type Mat4 = number[][]; // 4x4 matrix

TransformComponent: Represents local spatial data relative to a parent.

interface TransformComponent extends Component {
    position: Vec3;
    rotation: Quat;
    scale: Vec3;
    parent?: Entity;
}

WorldTransformComponent: A cached component for world-space transforms.

interface WorldTransformComponent extends Component {
    matrix: Mat4;
}

PhysicsTransformSystem: A system that updates transforms for physics calculations.

class PhysicsTransformSystem implements System {
    private signature: Signature;
    constructor(componentTypes: any[]) { /*...*/ }

    getSignature(): Signature { return this.signature; }

    /**
     * This system runs in the fixed loop and is responsible for updating
     * the WorldTransformComponent for physics input. It must handle the
     * parent-child hierarchy to ensure parent transforms are processed first.
     */
    update(entities: Entity[], deltaTime: number): void {
        // ...
    }

    onEntityDestroyed(entity: Entity): void {
        // ...
    }
}

RenderTransformSystem: A system that updates transforms for rendering.

class RenderTransformSystem implements System {
    private signature: Signature;
    constructor(componentTypes: any[]) { /*...*/ }

    getSignature(): Signature { return this.signature; }

    /**
     * This system runs in the variable loop and is responsible for updating
     * the WorldTransformComponent for rendering output, potentially with interpolation.
     */
    update(entities: Entity[], deltaTime: number): void {
        // ...
    }

    onEntityDestroyed(entity: Entity): void {
        // ...
    }
}

3. Physics & Interaction Module
3.1. Module Responsibility
This module handles physical simulation and publishes strongly typed events.

3.2. Type Definitions & Interfaces
PhysicsComponent: Pure data for physics properties.

interface PhysicsComponent extends Component {
    mass: number;
    materialProfile: string; // A reference to a material profile asset
    isTrigger: boolean;
}

PhysicsSystem: We must explicitly define the physics system's methods and responsibilities. It needs to find and update entities with both a PhysicsComponent and a WorldTransformComponent.

class PhysicsSystem implements System {
    // ...
    update(entities: Entity[], deltaTime: number): void {
        // 1. Iterate over entities with PhysicsComponent & WorldTransformComponent
        // 2. Perform collision detection and resolution
        // 3. Publish CollisionEvent for each collision
        // 4. Update the position of dynamic entities
    }
    onEntityDestroyed(entity: Entity): void {
        // ... Clean up any physics engine-specific resources for the destroyed entity.
    }
}

CollisionEvent: A generic collision event payload that can be published by any system.

interface CollisionEventPayload {
    entityA: Entity;
    entityB: Entity;
}

PhysicsCollisionEventPayload: A specific event payload published by the PhysicsSystem.

interface PhysicsCollisionEventPayload extends CollisionEventPayload {
    contactPoint: Vec3;
    relativeVelocity: Vec3;
    impulse: number;
    materials: [string, string];
}

4. Audio System Module
4.1. Module Responsibility
The core sound engine, managing playback and spatialization using data from other systems.

4.2. Type Definitions & Interfaces
SoundInstance: A top-level interface for the internal runtime state of an audio source.

interface SoundInstance {
    sourceHandle: any; // A handle for the low-level Resonance Audio source
    state: "playing" | "paused" | "stopped";
    assetId: string;
}

AudioListenerComponent: This component, attached to the camera or player entity, marks the listener. The AudioSystem's job is to find the entity with this component and feed its transform data to the spatial audio API's listener.

interface AudioListenerComponent extends Component {}

AudioSourceComponent: Static audio configuration data.

interface AudioSourceComponent extends Component {
    assetId: string;
    isSpatial?: boolean;
    spatialOptions?: SpatialAudioOptions;
    /** How important this sound is in voice-stealing decisions (higher wins). */
    priority?: number; // default 0..100, suggest 50 default
    /**Category for shared limits/ducking; e.g., "SFX", "Dialogue", "UI", "Ambience". */
    category?: string; // default "SFX"
    /** Concurrency rules */
    maxInstancesPerEntity?: number;    // e.g., limit footsteps spam
    voiceStealGroup?: string;          // group similar sounds for stealing (e.g., "footsteps")
    /**Optional distance-based priority scaling (helps explosions over far crickets).*/
    priorityAttenuation?: {
        model: "inverse_distance" | "none";
        min?: number;  // clamp
        max?: number;  // clamp
    };
}

AudioPlaybackStateComponent: Dynamic runtime data for a playing sound.

interface AudioPlaybackStateComponent extends Component {
    busName: string;
    isSpatial: boolean;
    volume: number;
    // A handle to the internal sound instance managed by the AudioSystem.
    soundInstanceHandle: any;
    /** Streaming-specific runtime */
    streamId?: number;       // if streaming
    streaming?: boolean;
}

SpatialAudioOptions: A key part of Resonance Audio is its ability to configure various spatialization properties. You need a data structure to hold these settings.

interface SpatialAudioOptions {
    // Defines the audio source's shape, e.g., omnidirectional or directional.
    // Resonance Audio uses `alpha` and `sharpness` for this.
    directivityPattern: { alpha: number; sharpness: number };

    // Sets how the volume changes with distance.
    // Resonance Audio provides different models for this.
    rolloffModel: "logarithmic" | "linear" | "none";

    // Defines how wide the source of the sound is.
    sourceWidth: number;

    // Defines the area where the sound is a constant volume before rolloff begins.
    minDistance: number;
    maxDistance: number;
}

IMixerProcessor: An interface for the AudioSystem to communicate with the low-level MixerService via a thread-safe queue.

interface IMixerProcessor {
    /** RT-safe, lock-free push; drops on overflow (with counter) to keep audio glitch-free. */
    push(cmd: MixerCommand): void;
    /**Current sample rate & buffer size for timestamping atFrame.*/
    getTiming(): { sampleRate: number; bufferFrames: number; streamTimeFrames: number };
}

AudioSystem: The AudioSystem needs methods to handle the life cycle of a sound source and to route it to the mixer. It would use the Resonance Audio API to manage the spatial properties of each source.

interface AudioSystem extends System {
    /**
     *The primary update loop. It subscribes to listener transform events and updates
     * audio sources based on their new positions and the listener's cached transform.
     */
    update(entities: Entity[], deltaTime: number): void;

    /**
     * Initializes the system and subscribes to events.
     */
    initialize(world: World): void;
    
    // Note: The methods below are private and are only ever called by event
    // handlers within the AudioSystem. No other system should call them directly.
    _onListenerTransformUpdated(payload: ListenerTransformEventPayload): void;
    _startPlayback(entity: Entity): void;
    _stopPlayback(entity: Entity): void;
    _pausePlayback(entity: Entity): void;
}

AudioListenerSystem: Finds and caches the listener's transform. It now explicitly publishes an event.

interface AudioListenerSystem extends System {
    /**
     *Updates and publishes the listener's transform.
     */
    update(entities: Entity[], deltaTime: number): void; // Entities with AudioListenerComponent & WorldTransformComponent
    onEntityDestroyed(entity: Entity): void;
}

5. Events & Communication Module
5.1. Module Responsibility
This module provides a robust, strongly-typed publish-subscribe system.

5.2. Type Definitions & Interfaces
EventType: An enum for strongly-typed event identifiers.

const enum EventType {
    COLLISION_EVENT = "collision",
    PHYSICS_COLLISION_EVENT = "physics_collision",
    AUDIO_REQUEST = "audio_request",
    PLAY_SOUND,
    STOP_SOUND,
    SET_SOUND_VOLUME,
    LISTENER_TRANSFORM_UPDATED = "listener_transform_updated", // New event for decoupling
    // Add more event types here
}

Event: The base event interface.

interface Event<T> {
    type: EventType;
    payload: T;
    priority?: number;
}

Event Payloads: These interfaces define the data carried by each event.

interface PlaySoundEventPayload {
    /**
     *The ID of the entity that contains the AudioSourceComponent.
     * The AudioSystem will use this ID to retrieve the necessary data from the component.
     */
    entityId: number;
}
interface StopSoundEventPayload {
    entityId: number;
}
interface SetVolumeEventPayload {
    entityId: number;
    volume: number;
}
interface ListenerTransformEventPayload {
    entity: Entity;
    transform: WorldTransformComponent;
}

EventBus: The core event bus needs to manage subscribers and a queue. The publishImmediate method has been removed for deterministic execution.

interface EventBus {
    publish<T>(event: Event<T>): void;
    subscribe<T>(eventType: EventType, handler: (event: Event<T>) => void): void;
}

```typescript
class EventBusImpl implements EventBus {
    private subscribers: Map<EventType, Array<(event: Event<any>) => void>>;
    private eventQueue: Event<any>[];
    // ... implementation
}

6. Resource & Asset Abstraction Module
6.1. Module Responsibility
Manages asset loading and caching asynchronously and non-blockingly. The design has been updated to be thread-safe.

6.2. Type Definitions & Interfaces
Asset: The base asset interface.

interface Asset {
    id: string;
    refCount: number;
    data: unknown; // The actual asset data
}

StreamedAudioAsset:

interface StreamedAudioAsset extends Asset {
    /** decoder-handle & format metadata */
    channels: number;
    sampleRate: number;
    durationSec: number;
    /** Streaming hints */
    streaming: true;
    preloadSeconds?: number; // default 1.0
    readAheadSeconds?: number; // default 3.0
    ringBufferSeconds?: number; // default 6.0
    loop?: boolean;
    loopStartSec?: number; // gapless loop points (samples ok too)
    loopEndSec?: number;
}

AssetComponent: A component to explicitly link an entity to a loaded asset.

interface AssetComponent extends Component {
    assetId: string;
    isLoaded: boolean;
}

ResourceLoader: The loader's internal state is now protected from race conditions by a command queue.

type ResourceCommand =
  | { type: "LOAD_ASYNC"; assetId: string }
  | { type: "RELEASE"; assetId: string };

interface ResourceLoader extends System {
    /**
     * Pushes a load command to a thread-safe queue. The promise resolves
     * when the asset is fully loaded and ready for use.
     */
    loadAsync<T extends Asset>(id: string): Promise<T>;
    /**
     * Pushes a release command to the queue, decrementing the asset's refCount.
     */
    release(id: string): void;
    getAsset<T extends Asset>(id: string): T | undefined;
    isLoaded(id: string): boolean;
    /**
     * Returns a stream id; data is produced on a decode threadpool into a ring buffer.
     */
    openAudioStream(id: string): Promise<number>; // returns streamId
    closeAudioStream(streamId: number): void;
    /**
     * For local desktop builds, enable memory-mapped I/O for large files.
     */
    setStreamingBackend(opts: {
        io: "memory_map" | "std_io" | "web_fetch";
        decodeThreads?: number;         // default: cores/2
        targetLatencyMs?: number;       // default: 60
    }): void;
    /**
     * The update method is now the single point of truth for processing all
     * commands from the queue and managing the internal state of assets,
     * such as decrementing reference counts or performing a garbage collection pass
     * for assets that are no longer in use.
     */
    update(entities: Entity[], deltaTime: number): void;
    onEntityDestroyed(entity: Entity): void; // The system handles cleanup on entity destruction.
}

7. Runtime & Flow Module
7.1. Module Responsibility
Manages the engine's heartbeat and ensures a stable hybrid timestep.

7.2. Type Definitions & Interfaces
GameLoop: A well-defined loop that handles the hybrid timestep.

interface GameLoop {
    start(): void;
    stop(): void;
    runFixedUpdate(deltaTime: number): void;
    runVariableUpdate(deltaTime: number): void;
}

GameLoop implementation.

class GameLoop {
    private world: World;
    private lastTime: number = 0;
    private accumulator: number = 0;
    private fixedDeltaTime: number;

    constructor(world: World, fixedDeltaTime: number = 1 / 60) {
        this.world = world;
        this.fixedDeltaTime = fixedDeltaTime;
    }

    start(): void {
        this.world.bootstrap(); // Now the sole point of initialization
        this.lastTime = performance.now();
        requestAnimationFrame(this.runFrame.bind(this));
    }

    private runFrame(currentTime: number): void {
        const deltaTime = (currentTime - this.lastTime) / 1000;
        this.lastTime = currentTime;
        this.accumulator += deltaTime;

        // Fixed update loop
        while (this.accumulator >= this.fixedDeltaTime) {
            this.runFixedUpdate(this.fixedDeltaTime);
            this.accumulator -= this.fixedDeltaTime;
        }

        // Variable update loop
        this.runVariableUpdate(deltaTime);
        requestAnimationFrame(this.runFrame.bind(this));
    }

    private runFixedUpdate(deltaTime: number): void {
        // Ordered system updates for fixed-rate logic.
        this.world.systemManager.runFixedUpdate(deltaTime);
    }

    private runVariableUpdate(deltaTime: number): void {
        // Rendering and other variable-rate systems.
        this.world.systemManager.runVariableUpdate(deltaTime);
    }
}

Update Order: The explicit, non-negotiable update order for a single frame. This is now enforced by the SystemManager's registration methods and internal ordered lists.

Fixed Update Loop:

PhysicsTransformSystem (order: 0)

PhysicsSystem (order: 1)

Variable Update Loop:

InputSystem (order: 0)

AudioListenerSystem (order: 1) // Now explicitly updates before AudioSystem

RenderTransformSystem (order: 2)

AmbientZoneSystem (order: 3)

AudioSystem (order: 4)

RenderingSystem (order: 5)
(Note: MixerService is not in this list as it is a real-time service)

8. Final Services
8.1. Serialization Service
SerializationService: A dedicated service to handle saving and loading the world state. It is not a System and does not run in the game loop.

interface SerializationService {
    saveGame(filePath: string, world: World): void;
    /**
     * Populates the existing World instance with saved data to avoid
     * engine state disruption and invalid references.
     */
    loadGame(filePath: string, world: World): void;
}

8.2. Mixer Service
MixerService: A service that manages the audio mix. It operates on a separate thread and is not a System in the ECS sense.

type MixerCommand =
  | { type: "StartVoice"; atFrame: number; bus: string; streamId: number; gain: number; pan?: number }
  | { type: "StopVoice";  atFrame: number; voiceId: number; fadeOutMs?: number }
  | { type: "SetVoiceGain"; atFrame: number; voiceId: number; gain: number }
  | { type: "SetBusGain"; atFrame: number; bus: string; gain: number }
  | { type: "AddBusEffect"; atFrame: number; bus: string; effect: AudioEffect }
  | { type: "RemoveBusEffect"; atFrame: number; bus: string; effectId: number }
  | { type: "Sidechain"; atFrame: number; rule: DuckingRule };

interface DuckingRule {
    /** e.g., Dialogue ducks Music by 10 dB with 20ms attack, 200ms release */
    sidechainSourceCategory: string; // "Dialogue"
    targetBus: string;               // "Music"
    duckAmountDb: number;            // e.g., 10
    attackMs: number;                // e.g., 20
    releaseMs: number;               // e.g., 200
    holdMs?: number;                 // optional
    thresholdRms?: number;           // only engage if source above threshold
}

interface MixerService {
    /** Global & per-bus voice caps. */
    setMaxVoicesGlobal(max: number): void;        // e.g., 64
    setMaxVoicesForBus(busName: string, max: number): void; // e.g., Music=4, Dialogue=8
    /** Voice stealing strategy. */
    setVoiceStealPolicy(policy: "lowest_priority" | "oldest" | "quietest_rms" | "hybrid"): void;
    /** Concurrency rules by group or category. */
    setMaxConcurrentInGroup(group: string, max: number): void;
    setMaxConcurrentInCategory(category: string, max: number): void;
    /** Query for debugging. */
    getActiveVoices(): Array<{
        id: number; bus: string; assetId: string; priorityScore: number; rms: number; ageMs: number;
    }>;
    /** Returns a handle to the mixer's processor for RT-safe command pushing. */
    getProcessor(): IMixerProcessor;
}

9. Extensions & Advanced Features
9.1. Spatial Queries / Orientation
Purpose: Give devs precise, accessibility-friendly info about where things are, how occluded they are, and quick helpers for narration/UI.

interface SpatialQueryService {
    /** Returns azimuth (deg, +right/-left), elevation (deg), and distance (m) from listener to entity. */
    getPolar(entity: Entity, listener: Entity): { azimuth: number; elevation: number; distance: number };
    /** Convenience: already formatted for TTS (“30° left, 5 meters”). */
    getPolarLabel(entity: Entity, listener: Entity, opts?: { units?: "m" | "ft"; precision?: number }): string;
    /** Returns normalized [0..1] occlusion factor; 0 = clear LOS, 1 = fully occluded. */
    getOcclusion(entity: Entity, listener: Entity): number;
    /** Nearest N entities by distance (filtered by optional component signature). */
    nearestEntities(listener: Entity, max: number, signature?: Signature): Array<{ entity: Entity; distance: number }>;
    /** Entities within cone from the listener’s forward (useful for “what’s ahead?” queries). */
    inAuditoryCone(listener: Entity, angleDeg: number, maxDistance: number, signature?: Signature): Entity[];
}

Implementation Notes:

Uses WorldTransformComponent (+ optional physics/scene BVH) to compute distances/angles.

getOcclusion can be a fast raycast against physics colliders; if none, fall back to simple room/portal heuristics.

Registered as a Service on World (not a System).

9.2. Ambient Zones & Layered Soundscapes
Components

type Shape =
    | { type: "sphere"; center: Vec3; radius: number }
    | { type: "aabb"; min: Vec3; max: Vec3 }
    | { type: "box"; center: Vec3; size: Vec3; rotation?: Quat };

interface AmbientZoneComponent extends Component {
    shape: Shape;
    /** Higher wins if zones overlap. */
    priority?: number; // default 0..10
    /** Crossfade time when entering/leaving this zone. */
    crossfadeMs?: number; // default 500
}

interface AmbientLayer {
    assetId: string;
    busName?: string;          // default "Ambience"
    loop?: boolean;            // default true
    volume: number;            // base volume (0..1)
    /** Optional time-of-day weight; times in 24h local or game clock. */
    daypart?: Array<{ startHour: number; endHour: number; weight: number }>;
    /** Random start offset to avoid phasing when multiple zones use same asset. */
    randomStart?: boolean;
}

interface AmbientProfileComponent extends Component {
    layers: AmbientLayer[];
}

System

class AmbientZoneSystem implements System {
    getSignature(): Signature { /* AmbientZoneComponent | AmbientProfileComponent */ }
    update(entities: Entity[], dt: number): void {
        // 1) Find listener
        // 2) For each zone, compute membership (inside?, distance to boundary for pre/post roll)
        // 3) Resolve overlaps by priority
        // 4) For active profile, ensure layers are started (via Mixer command queue) and crossfaded
        // 5) Apply daypart weights smoothly
    }
    onEntityDestroyed(e: Entity): void { /* fade out layers */ }
}

9.3. Dev Ergonomics & Debugging (Blind-Friendly)
TTS-Friendly Debug Channel

const enum DebugEventType {
    AUDIO_PLAY = "audio_play",
    AUDIO_STOP = "audio_stop",
    COLLISION = "collision",
    ZONE_ENTER = "zone_enter",
    ZONE_EXIT = "zone_exit",
}

interface DebugEvent<T> { type: DebugEventType; payload: T; }

interface DebugService {
    publish<T>(ev: DebugEvent<T>): void;
    subscribe(type: DebugEventType, handler: (ev: DebugEvent<any>) => void): void;
    /** Optional: speak via OS TTS for blind devs while testing. */
    setTTSEnabled(enabled: boolean): void;
}

Examples emitted by systems:

AudioSystem on start/stop

PhysicsSystem on collision (entity names, contact point)

AmbientZoneSystem on enter/exit with zone name

Minimal Scripting (Optional but powerful)

interface BootstrapScriptingService {
    /** Spawn entities from a simple JSON/YAML bundle. */
    spawnFromData(data: unknown): Entity[];
}
