use std::sync::{Arc, atomic::{AtomicU64, Ordering}, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig, SampleFormat};
use crossbeam_channel::{unbounded, Sender, Receiver};

use crate::{BackendError, RenderFn, DeviceInfo, AudioBackend, DiagnosticEvent, DiagnosticsCb};

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
    // Render function stored behind a Mutex for now (simpler semantics).
    // TODO: replace with ArcSwapOption for RT lock-free access.
    render: Arc<Mutex<Option<crate::RenderFn>>>,
    // Atomic frame counter updated by worker.
    frames: AtomicU64,
    // Diagnostics callback (worker uses this via a clone of the Arc)
    diagnostics: Option<DiagnosticsCb>,
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
        };

        let (tx, rx) = unbounded::<CtrlMsg>();

        let inner = Arc::new(CpalBackendInner {
            info,
            render: Arc::new(Mutex::new(None)),
            frames: AtomicU64::new(0),
            diagnostics: None,
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
    // Preallocated conversion buffer (per-callback); grow on demand.
    let mut _conv_buf: Vec<f32> = Vec::new();
    let channels = config.channels as usize;

    // Worker local diagnostics clone for non-RT callbacks.
    let mut diagnostics = inner.diagnostics.clone();

    let mut stream_opt: Option<cpal::Stream> = None;

    loop {
        // Non-blocking handle of control messages so the worker can manage stream lifecycle.
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CtrlMsg::SetRender(opt) => {
                    // opt is Option<RenderFn> where RenderFn == Arc<dyn Fn...>
                    let mut g = inner.render.lock().unwrap();
                    *g = opt;
                }
                CtrlMsg::Start => {
                    if stream_opt.is_none() {
                        // Build stream with RT-safe callback.
                        // Clone `inner` specifically for the closure to prevent the move error.
                        let inner_for_cb = inner.clone();
                        let channels_local = channels;
                        let sample_rate = config.sample_rate.0;

                        // Clone the diagnostics callback for this stream's error callback
                        let diagnostics_for_err_cb = diagnostics.clone();
                        let err_cb = move |err| {
                            // Non-RT: report XRUN via diagnostics callback if set.
                            eprintln!("CPAL stream error: {}", err);
                            if let Some(cb) = &diagnostics_for_err_cb {
                                let cb_clone = cb.clone();
                                std::thread::spawn(move || cb_clone(DiagnosticEvent::XRun { count: 1 }));
                            }
                        };

                        // The data callback uses the cloned `inner_for_cb`.
                        let data_cb = move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                            let opt_render = {
                                // Use the cloned `inner_for_cb` to get a guard on the render function.
                                let guard = inner_for_cb.render.lock().unwrap();
                                guard.clone()
                            };
                            if let Some(render) = opt_render.as_ref() {
                                let frames = data.len() / channels_local;
                                // Call user render; protect from panic.
                                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    (render)(data, sample_rate, frames);
                                }));
                                if res.is_err() {
                                    data.iter_mut().for_each(|s| *s = 0.0);
                                }
                            } else {
                                data.iter_mut().for_each(|s| *s = 0.0);
                            }

                            // Update frames counter (relaxed)
                            let frames_written = (data.len() / channels_local) as u64;
                            inner_for_cb.frames.fetch_add(frames_written, Ordering::Relaxed);
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
                                if let Some(cb) = &diagnostics {
                                    let cb_clone = cb.clone();
                                    std::thread::spawn(move || cb_clone(DiagnosticEvent::Other(format!("stream build failed: {}", e))));
                                }
                            }
                        }
                    }
                }
                CtrlMsg::Stop => {
                    stream_opt = None; // drop stream to stop
                }
                CtrlMsg::SetDiagnostics(cb) => {
                    diagnostics = cb;
                }
                CtrlMsg::Shutdown => {
                    // The stream will be dropped automatically when the function returns and stream_opt
                    // goes out of scope.
                    return;
                }
            }
        }

        // Sleep briefly to yield; worker is event-driven via channels.
        thread::sleep(Duration::from_millis(2));
    }
}

impl AudioBackend for CpalAudioBackend {
    fn start(&mut self, render: RenderFn) -> Result<(), BackendError> {
        // Set the render function and send Start.
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
}
