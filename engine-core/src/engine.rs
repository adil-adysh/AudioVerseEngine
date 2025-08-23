use bevy_ecs::prelude::*;

use crate::components::*;
use crate::events::*;

pub struct Engine {
    pub world: World,
    pub fixed_schedule: Schedule,
    pub variable_schedule: Schedule,
}

impl Engine {
    pub fn new() -> Self {
        let mut world = World::new();
    init_event_resources(&mut world);

        let mut fixed_schedule = Schedule::default();
        let mut variable_schedule = Schedule::default();

        // Systems will be added by consumers / bootstrap code later.

        Self {
            world,
            fixed_schedule,
            variable_schedule,
        }
    }

    /// Called once to register engine-provided systems and do one-time setup.
    pub fn bootstrap(&mut self) {
    // Note: integration with `audio-system` (registering its Bevy systems)
    // should be performed by the application (for example in `app-windows`).
    // This avoids a cyclic path dependency between engine-core and audio-system.
    }

    /// Expose mutable access to fixed schedule so callers can register systems.
    pub fn fixed_schedule_mut(&mut self) -> &mut Schedule {
        &mut self.fixed_schedule
    }

    /// Expose mutable access to variable schedule so callers can register systems.
    pub fn variable_schedule_mut(&mut self) -> &mut Schedule {
        &mut self.variable_schedule
    }

    pub fn create_entity(&mut self) -> Entity {
        self.world.spawn_empty().id()
    }

    pub fn destroy_entity(&mut self, e: Entity) {
        self.world.despawn(e);
    }

    pub fn add_sound(&mut self, entity: Entity, asset_id: impl Into<String>) {
        let src = AudioSourceComponent {
            asset_id: asset_id.into(),
            is_spatial: false,
            priority: 50,
            category: "SFX".to_string(),
        };
        if let Some(mut e) = self.world.get_entity_mut(entity) {
            e.insert(src);
        }
    }

    pub fn play(&mut self, entity: Entity) {
        let mut events = self.world.resource_mut::<Events<PlaySoundEvent>>();
        events.send(PlaySoundEvent { entity });
    }

    /// Runs fixed update then variable update once with the provided delta (seconds)
    pub fn update(&mut self, _delta: f32) {
        self.fixed_schedule.run(&mut self.world);
        self.variable_schedule.run(&mut self.world);
    }
}
