use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), audio_backend::BackendError> {
    // Parse optional CLI args: <duration_ms> <freq>
    let mut args = std::env::args().skip(1);
    let duration_ms: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(2000);
    let freq: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(440);

    // Create our audio-system with a mixer capacity and typical params.
    let sys = Arc::new(
        audio_system::AudioSystem::new(64, 48000, 128)
            .map_err(|e| audio_backend::BackendError::Other(e.to_string()))?,
    );
    sys.initialize();

    // Create backend and start it with the render function using the same shared system.
    let mut backend = audio_backend::create_audio_backend()?;
    let render_generic = audio_system::render_fn_for_system(Arc::clone(&sys));
    // audio-backend's RenderFn type is Arc<dyn Fn(&mut [f32], u32, usize) + Send + Sync>
    let render: audio_backend::RenderFn = render_generic.clone();
    backend.start(render)?;

    // Start a sine source via the same system so it's audible; use asset id "sine:<freq>".
    let src = audio_system::AudioSourceComponent {
        asset_id: format!("sine:{}", freq),
        is_spatial: false,
        spatial_options: None,
        priority: 50,
        category: "SFX".to_string(),
    };
    let _handle = sys.start_playback(&src);

    // Let it run for the requested duration to audibly verify the sine source.
    std::thread::sleep(Duration::from_millis(duration_ms));

    backend.stop()?;
    Ok(())
}
