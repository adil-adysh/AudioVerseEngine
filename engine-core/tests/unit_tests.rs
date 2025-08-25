use bevy::prelude::{Vec3, Transform, GlobalTransform, MinimalPlugins};
use bevy_app::{App, Update, PostUpdate};

use engine_core::{Aabb, Sphere, Player, Velocity, doppler_effect_system, audio_occlusion_system};

#[test]
fn test_aabb_and_sphere_contains() {
    let aabb = Aabb { min: Vec3::new(-1.0, 0.0, -1.0), max: Vec3::new(1.0, 2.0, 1.0) };
    let sphere = Sphere { center: Vec3::new(0.0, 1.0, 0.0), radius: 2.0 };

    assert!(aabb.contains(Vec3::new(0.0, 1.0, 0.0)));
    assert!(!aabb.contains(Vec3::new(5.0, 5.0, 5.0)));

    assert!(sphere.contains(Vec3::new(1.0, 1.0, 0.0)));
    assert!(!sphere.contains(Vec3::new(3.0, 1.0, 0.0)));
}

#[test]
fn test_doppler_updates_playback_speed() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // Spawn listener (Player) with Transform and Velocity
    let _listener = app.world_mut().spawn((Player, Transform::from_translation(Vec3::ZERO), Velocity(Vec3::new(0.0, 0.0, 0.0)))).id();

    // Spawn emitter with PlaybackSettings, GlobalTransform and Velocity
    let playback = bevy_audio::PlaybackSettings::default();
    let emitter_transform = GlobalTransform::from(Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)));
    let emitter = app.world_mut().spawn((playback, emitter_transform, Velocity(Vec3::new(-1.0, 0.0, 0.0)))).id();

    // Register system
    app.add_systems(Update, doppler_effect_system);
    app.update();

    // Check playback settings changed on the emitter
    let settings = app.world().get::<bevy_audio::PlaybackSettings>(emitter).unwrap();
    assert!(settings.speed != 1.0);
}

#[test]
fn test_audio_occlusion_fallback_no_rapier() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // Spawn listener
    app.world_mut().spawn((Player, Transform::from_translation(Vec3::ZERO), Velocity(Vec3::ZERO)));

    // Spawn emitter with PlaybackSettings and GlobalTransform
    let playback = bevy_audio::PlaybackSettings::default();
    let emitter_transform = GlobalTransform::from(Transform::from_translation(Vec3::new(5.0, 0.0, 0.0)));
    let emitter = app.world_mut().spawn((playback, emitter_transform)).id();

    // Register occlusion system which will try to access RapierContext; with no Rapier present it should not panic and should leave muted=false
    app.add_systems(PostUpdate, audio_occlusion_system);
    app.update();

    let settings = app.world().get::<bevy_audio::PlaybackSettings>(emitter).unwrap();
    assert_eq!(settings.muted, false);
}
