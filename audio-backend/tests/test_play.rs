use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[test]
fn test_play_sine_wave() {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No output device available");
    let config = device.default_output_config().unwrap();

    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    let mut sample_clock = 0f32;
    let freq = 440.0;
    let amplitude = 0.2;

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| {
                for frame in data.chunks_mut(channels) {
                    let value = (sample_clock * freq * 2.0 * std::f32::consts::PI).sin() * amplitude;
                    for sample in frame {
                        *sample = value;
                    }
                    sample_clock = (sample_clock + 1.0 / sample_rate) % 1.0;
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_output_stream(
            &config.into(),
            move |data: &mut [i16], _| {
                for frame in data.chunks_mut(channels) {
                    let value = ((sample_clock * freq * 2.0 * std::f32::consts::PI).sin() * amplitude * i16::MAX as f32) as i16;
                    for sample in frame {
                        *sample = value;
                    }
                    sample_clock = (sample_clock + 1.0 / sample_rate) % 1.0;
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_output_stream(
            &config.into(),
            move |data: &mut [u16], _| {
                for frame in data.chunks_mut(channels) {
                    let value = (((sample_clock * freq * 2.0 * std::f32::consts::PI).sin() * amplitude + 1.0) * 0.5 * u16::MAX as f32) as u16;
                    for sample in frame {
                        *sample = value;
                    }
                    sample_clock = (sample_clock + 1.0 / sample_rate) % 1.0;
                }
            },
            err_fn,
            None,
        ),
        _ => panic!("Unsupported sample format"),
    }.expect("Failed to build output stream");

    stream.play().expect("Failed to play stream");
    std::thread::sleep(std::time::Duration::from_secs(2));
}
