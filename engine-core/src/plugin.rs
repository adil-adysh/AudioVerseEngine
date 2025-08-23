use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoSystemConfigs;

/// Bevy App plugin that registers engine-core resources and systems.
/// This allows running the engine inside a Bevy App runner while
/// keeping the existing Engine facade intact for non-App usage.
pub struct EngineCorePlugin;

impl Plugin for EngineCorePlugin {
    fn build(&self, app: &mut App) {
        // Register events and baseline resources (idempotent)
        crate::events::init_event_resources(&mut app.world);
        if app
            .world
            .get_resource::<crate::components::SpaceGraph>()
            .is_none()
        {
            app.world
                .insert_resource(crate::components::SpaceGraph::default());
        }
        // Physics resources
        if app
            .world
            .get_resource::<crate::physics::PhysicsResources>()
            .is_none()
        {
            app.world
                .insert_resource(crate::physics::PhysicsResources::default());
        }

        // Update schedule: staged smaller chains
        app.add_systems(Update, (crate::systems::set_update_timestep_system, crate::systems::navigation_system, crate::transform::physics_transform_system).chain());
        app.add_systems(Update, (crate::systems::navigation_step_system, crate::physics::physics_spawn_system, crate::physics::physics_step_system).chain());
        app.add_systems(Update, (crate::transform::render_transform_system, crate::systems::audio_listener_system, crate::systems::audio_system).chain());
        app.add_systems(Update, (crate::transform::despawn_cleanup_system, crate::systems::space_graph_index_system).chain());
        app.add_systems(Update, (crate::systems::navmesh_boundary_cues_system, crate::systems::navmesh_wayfinding_cues_system, crate::systems::space_membership_system).chain());
    }
}
