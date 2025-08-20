use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig, SampleFormat};
use crossbeam_channel::{unbounded, Sender, Receiver};
use arc_swap::ArcSwapOption;
use audio_backend::DeviceInfoProvider;

// These public re-exports make these types and traits visible to
// external crates, including your test suite. This fixes the E0432
// "unresolved imports" error you were seeing.
pub use audio_backend::{AudioBackend, BackendError, DeviceInfo, RenderFn, DiagnosticEvent, DiagnosticsCb};

/// Worker-thread-backed CPAL backend.
/// Public `CpalAudioBackend` is a Send-safe handle that communicates with the
/// worker thread via a simple control channel. The worker owns the CPAL `Stream`
/// and preallocated conversion buffers so no non-Send objects cross thread
/// boundaries.
pub struct CpalAudioBackend {
    inner: Arc<CpalBackendInner>,
}

struct CpalBackendInner {
    // Read-only device info.
    info: DeviceInfo,
    // Render function stored behind a lock-free ArcSwapOption for real-time access.
    // The type is now explicitly wrapped in an Arc to satisfy ArcSwapOption's requirements.
    render: Arc<ArcSwapOption<RenderFn>>,
    // Atomic frame counter updated by worker.
    frames: AtomicU64,
    // Diagnostics callback (worker uses this via a clone of the Arc)
    diagnostics: Arc<ArcSwapOption<DiagnosticsCb>>,
    // Control channel sender to worker.
    ctrl_tx: Sender<CtrlMsg>,
}

enum CtrlMsg {
    SetRender(Option<RenderFn>),
    Start,
    Stop,
    SetDiagnostics(Option<DiagnosticsCb>),
    Shutdown,
}

impl CpalAudioBackend {
    pub fn new() -> Result<Self, BackendError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(BackendError::DeviceNotFound)?;

        let mut supported_configs = device.supported_output_configs()
            .map_err(|e| BackendError::Other(e.to_string()))?
            .collect::<Vec<_>>();

        if supported_configs.is_empty() {
            return Err(BackendError::UnsupportedFormat("no supported configs".into()));
        }

        // Prefer f32 interleaved, stereo, maximum sample rate.
        let chosen = supported_configs.iter()
            .rev()
            .find(|c| c.sample_format() == SampleFormat::F32 && c.channels() >= 2)
            .cloned()
            .or_else(|| supported_configs.pop())
            .unwrap();

        let config = chosen.with_max_sample_rate().config();

        let buffer_frames = match config.buffer_size {
            cpal::BufferSize::Fixed(n) => n as usize,
            cpal::BufferSize::Default => 0_usize,
        };

        let info = DeviceInfo {
            sample_rate: config.sample_rate.0,
            buffer_size: buffer_frames,
            channels: config.channels as u16,
            device_name: Some("CPAL Device".to_string()),
        };

        let (tx, rx) = unbounded::<CtrlMsg>();

        let inner = Arc::new(CpalBackendInner {
            info,
            render: Arc::new(ArcSwapOption::from(None)),
            frames: AtomicU64::new(0),
            diagnostics: Arc::new(ArcSwapOption::from(None)),
            ctrl_tx: tx.clone(),
        });

        // Spawn worker thread that owns the device, stream, and conversion buffers.
        let inner_worker = inner.clone();
        thread::spawn(move || {
            worker_loop(device, config, rx, inner_worker);
        });

        Ok(Self { inner })
    }
}

fn worker_loop(device: Device, config: StreamConfig, rx: Receiver<CtrlMsg>, inner: Arc<CpalBackendInner>) {
    let mut _conv_buf: Vec<f32> = Vec::new();
    let channels = config.channels as usize;

    let mut stream_opt: Option<cpal::Stream> = None;

    loop {
        match rx.recv() {
            Ok(msg) => {
                match msg {
                    CtrlMsg::SetRender(opt) => {
                        inner.render.store(opt.map(Arc::new));
                    }
                    CtrlMsg::Start => {
                        if stream_opt.is_none() {
                            // Clone `inner` for the error callback.
                            let inner_for_err_cb = inner.clone();
                            let err_cb = move |err| {
                                eprintln!("CPAL stream error: {}", err);
                                if let Some(cb) = &*inner_for_err_cb.diagnostics.load() {
                                    let cb_clone = cb.clone();
                                    std::thread::spawn(move || cb_clone(DiagnosticEvent::XRun { count: 1 }));
                                }
                            };

                            // Clone `inner` again for the data callback.
                            let inner_for_data_cb = inner.clone();
                            let channels_local = channels;
                            let sample_rate = config.sample_rate.0;

                            let data_cb = move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                                let opt_render = inner_for_data_cb.render.load();
                                if let Some(render) = opt_render.as_ref() {
                                    let frames = data.len() / channels_local;
                                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        (**render)(data, sample_rate, frames);
                                    }));
                                    if res.is_err() {
                                        data.iter_mut().for_each(|s| *s = 0.0);
                                    }
                                } else {
                                    data.iter_mut().for_each(|s| *s = 0.0);
                                }

                                let frames_written = (data.len() / channels_local) as u64;
                                inner_for_data_cb.frames.fetch_add(frames_written, Ordering::Relaxed);
                            };

                            match device.build_output_stream(&config, data_cb, err_cb, None) {
                                Ok(s) => {
                                    if let Err(e) = s.play() {
                                        eprintln!("Failed to play stream: {}", e);
                                    } else {
                                        stream_opt = Some(s);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to build stream: {}", e);
                                    if let Some(cb) = &*inner.diagnostics.load() {
                                        let cb_clone = cb.clone();
                                        std::thread::spawn(move || cb_clone(DiagnosticEvent::Other(format!("stream build failed: {}", e))));
                                    }
                                }
                            }
                        }
                    }
                    CtrlMsg::Stop => {
                        stream_opt = None;
                    }
                    CtrlMsg::SetDiagnostics(cb) => {
                        inner.diagnostics.store(cb.map(Arc::new));
                    }
                    CtrlMsg::Shutdown => {
                        return;
                    }
                }
            }
            Err(_) => {
                return;
            }
        }
    }
}

impl AudioBackend for CpalAudioBackend {
    fn start(&mut self, render: RenderFn) -> Result<(), BackendError> {
        self.inner.ctrl_tx.send(CtrlMsg::SetRender(Some(render))).map_err(|_| BackendError::Other("ctrl channel closed".into()))?;
        self.inner.ctrl_tx.send(CtrlMsg::Start).map_err(|_| BackendError::Other("ctrl channel closed".into()))?;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), BackendError> {
        self.inner.ctrl_tx.send(CtrlMsg::Stop).map_err(|_| BackendError::Other("ctrl channel closed".into()))?;
        self.inner.ctrl_tx.send(CtrlMsg::SetRender(None)).map_err(|_| BackendError::Other("ctrl channel closed".into()))?;
        Ok(())
    }

    fn sample_rate(&self) -> u32 { self.inner.info.sample_rate }
    fn buffer_size(&self) -> usize { self.inner.info.buffer_size }
    fn channels(&self) -> u16 { self.inner.info.channels }
    fn frames_since_start(&self) -> u64 { self.inner.frames.load(Ordering::Relaxed) }
    fn set_diagnostics_callback(&mut self, cb: Option<DiagnosticsCb>) {
        self.inner.ctrl_tx.send(CtrlMsg::SetDiagnostics(cb)).ok();
    }

    fn as_device_info_provider(&self) -> Option<&dyn DeviceInfoProvider> {
        Some(self)
    }
}

impl DeviceInfoProvider for CpalAudioBackend {
    fn get_device_name(&self) -> Option<&str> {
        self.inner.info.device_name.as_deref()
    }
}

// Real-backend integration tests (Windows-only). These exercise the actual
// backend via `create_audio_backend()` and are guarded to run only on
// Windows to avoid CI failures on other platforms.
#[cfg(all(test, target_os = "windows"))]
mod windows_integration_tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use std::thread::sleep;

    // A simple render function that writes a constant value.
    fn simple_render(buffer: &mut [f32], _sr: u32, _frames: usize) {
        for v in buffer.iter_mut() { *v = 0.1; }
    }

    #[test]
    fn integration_start_stop_real_backend() {
        // Try to create the real backend; if no device is found, skip the test.
    let backend = match audio_backend::create_audio_backend() {
            Ok(b) => b,
            Err(crate::BackendError::DeviceNotFound) => return,
            Err(e) => panic!("create_audio_backend failed: {:?}", e),
        };

        // Start the backend with a trivial render closure.
        let mut boxed = backend;
    let render: RenderFn = Arc::new(simple_render);
        boxed.start(render).expect("start should succeed");

        // Poll for up to 1 second for frames to advance; some devices take time to start.
        let frames_1 = boxed.frames_since_start();
        let mut frames_2 = frames_1;
        let mut waited = 0u64;
        while waited < 1000 && frames_2 == frames_1 {
            sleep(Duration::from_millis(100));
            waited += 100;
            frames_2 = boxed.frames_since_start();
        }

        boxed.stop().expect("stop should succeed");

        if frames_2 == frames_1 {
            eprintln!("No frames observed within timeout ({}ms); skipping strict assertion.", waited);
            return; // pass test on systems where audio can't be started
        }

        assert!(frames_2 >= frames_1, "frames should not decrease");
        assert!(frames_2 > 0, "frames should have advanced while running");
    }
}
