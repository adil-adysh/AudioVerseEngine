use bevy_ecs::prelude::*;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{terminal, ExecutableCommand};
use glam::Vec3;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};

use audio_backend::create_audio_backend;
use audio_system::render_fn_for_system;
use engine_core::engine::Engine;

fn main() -> std::io::Result<()> {
    // Prepare terminal raw mode for non-blocking input
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(crossterm::cursor::Hide)?;
    stdout.execute(terminal::EnterAlternateScreen)?;

    // Engine + audio setup
    let mut engine = Engine::new();
    engine.bootstrap();
    engine_audio::setup_audio_system(&mut engine.world, 48_000, 2, 128);
    let sys = engine.world.resource::<engine_audio::AudioSystemRes>().0.clone();
    let mut backend = create_audio_backend().expect("audio backend");
    backend.start(render_fn_for_system(sys.clone())).expect("start backend");

    // Entities
    let listener = engine.create_entity();
    engine.set_listener(listener);
    engine.set_position(listener, Vec3::new(0.0, 1.6, 0.0));

    let source = engine.create_entity();
    engine.add_sound(source, "sine:440");
    engine.set_position(source, Vec3::new(1.0, 1.6, 0.0));
    engine.play(source);

    // Game loop: ~60 FPS, handle arrows; Ctrl+C exits
    let mut pos = Vec3::new(0.0, 1.6, 0.0);
    let mut last = Instant::now();
    let target_frame = Duration::from_micros(16_667);

    'running: loop {
        // Drain all pending events quickly
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)) || key.code == KeyCode::Esc {
                    break 'running;
                }
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Up => pos.z -= 0.2,
                        KeyCode::Down => pos.z += 0.2,
                        KeyCode::Left => pos.x -= 0.2,
                        KeyCode::Right => pos.x += 0.2,
                        _ => {}
                    }
                }
            }
        }

        engine.set_position(listener, pos);

        // Render simple HUD
        stdout.execute(terminal::Clear(terminal::ClearType::All))?;
        writeln!(stdout, "AudioVerse Engine CLI")?;
        writeln!(stdout, "Use arrow keys to move listener. Ctrl+C or Esc to exit.")?;
        writeln!(stdout, "Listener position: x={:.2} y={:.2} z={:.2}", pos.x, pos.y, pos.z)?;
        stdout.flush().ok();

        // Step engine
        let now = Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;
        engine.update(dt);

        // Frame pacing
        let elapsed = last.elapsed();
        if elapsed < target_frame { std::thread::sleep(target_frame - elapsed); }
    }

    // Cleanup terminal
    stdout.execute(terminal::LeaveAlternateScreen)?;
    stdout.execute(crossterm::cursor::Show)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
