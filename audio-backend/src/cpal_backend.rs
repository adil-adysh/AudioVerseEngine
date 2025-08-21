use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::thread::{self, JoinHandle};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig, SampleFormat};
use crossbeam_channel::{unbounded, Sender, Receiver};
use arc_swap::ArcSwapOption;

use crate::{BackendError, RenderFn, DeviceInfo, AudioBackend, DiagnosticEvent, DiagnosticsCb, DeviceInfoProvider};

/// Worker-thread-backed CPAL backend.
/// Public `CpalAudioBackend` is a Send-safe handle that communicates with the
/// worker thread via a simple control channel. The worker owns the CPAL `Stream`
/// and preallocated conversion buffers so no non-Send objects cross thread
/// boundaries.
pub struct CpalAudioBackend {
    inner: Arc<CpalBackendInner>,
    // The key addition: a handle to the worker thread.
    // We use Option<JoinHandle> so we can take it out of the struct in the `Drop` implementation.
    thread_handle: Option<JoinHandle<()>>,
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

            // Prefer the device's default output config when available. This matches
            // how simple examples (like `play_tone`) pick the OS-configured default
            // and avoids surprising sample-rate differences.
            let config = match device.default_output_config() {
                Ok(default_cfg) => {
                    eprintln!("Using device.default_output_config(): {:?}", default_cfg);
                    default_cfg.config()
                }
                Err(_) => {
                    // Fallback: enumerate supported configs and pick an f32 stereo one.
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

                    chosen.with_max_sample_rate().config()
                }
            };

        let buffer_frames = match config.buffer_size {
            cpal::BufferSize::Fixed(n) => n as usize,
            cpal::BufferSize::Default => 0_usize,
        };

        let info = DeviceInfo {
            sample_rate: config.sample_rate.0,
            buffer_size: buffer_frames,
            channels: config.channels as u16,
            device_name: device.name().ok(),
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
        // The `thread::spawn` function returns a `JoinHandle`. We must store it.
        let handle = thread::spawn(move || {
            worker_loop(device, config, rx, inner_worker);
        });

        Ok(Self { 
            inner, 
            thread_handle: Some(handle), // Store the handle
        })
    }
}

// Implements the Drop trait to ensure the worker thread is gracefully shut down.
// This is crucial for preventing the race condition and the access violation.
impl Drop for CpalAudioBackend {
    fn drop(&mut self) {
        // Send a shutdown message to the worker thread.
        // We use `.ok()` to ignore the error in case the channel is already closed,
        // which might happen if the worker thread panicked.
        self.inner.ctrl_tx.send(CtrlMsg::Shutdown).ok();

        // Take the thread handle out of the struct and join the thread.
        // `self.thread_handle.take()` replaces the Option with None and returns the Some value,
        // which allows us to consume the handle.
        if let Some(handle) = self.thread_handle.take() {
            // Wait for the thread to finish. This is a blocking call.
            // This guarantees the worker has exited before `inner` is dropped.
            if let Err(e) = handle.join() {
                // Handle the case where the thread panicked.
                eprintln!("Worker thread panicked: {:?}", e);
            }
        }
    }
}

fn worker_loop(device: Device, config: StreamConfig, rx: Receiver<CtrlMsg>, inner: Arc<CpalBackendInner>) {
    let mut _conv_buf: Vec<f32> = Vec::new();
    let channels = config.channels as usize;

    let mut stream_opt: Option<cpal::Stream> = None;

    loop {
        // We block on `rx.recv()` here, which is fine because this is a dedicated worker thread.
        // `recv()` will return an error if the channel is disconnected (i.e., `CpalAudioBackend` is dropped and all senders are gone).
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
                    // Exit the loop and the thread.
                    CtrlMsg::Shutdown => {
                        return;
                    }
                }
            }
            // If the channel is disconnected (all senders dropped), the worker should exit.
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
