use resonance_audio_engine::{Renderer, Spatializer};

#[test]
fn spatializer_create_feed_and_destroy() {
    let mut r = Renderer::new(48000, 2, 64);
    // create spatializer which will create a source internally
    let mut s = Spatializer::new(&mut r, resonance_cxx::RenderingMode::kStereoPanning);

    // feed interleaved audio
    let audio = vec![0.1f32; 64 * 2];
    s.feed_interleaved(&audio, 2, 64);

    // feed planar audio
    let left = vec![0.2f32; 64];
    let right = vec![0.3f32; 64];
    let channels: [&[f32]; 2] = [&left, &right];
    let ok = s.feed_planar(&channels, 64);
    assert!(ok, "planar feed should succeed with matching lengths");

    // setters
    s.set_gain(0.7);
    s.set_distance_rolloff(resonance_cxx::DistanceRolloffModel::kLogarithmic);
    s.set_pose(1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 1.0);
    s.set_room_effects_gain(0.5);
    s.set_distance_attenuation(2.0);

    // destroy should call Api destroy
    s.destroy();
}
