use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use crate::{BackendError, RenderFn, DeviceInfo, AudioBackend, DiagnosticsCb};
use crate::DeviceInfoProvider;

/// A Send-safe mock audio backend using arc-swap for RT-safe render access.
pub struct MockAudioBackend {
    info: DeviceInfo,
    // Simple Mutex-protected optional render function for now.
    render: Arc<Mutex<Option<crate::RenderFn>>>,
    frames: AtomicU64,
    diagnostics: Option<DiagnosticsCb>,
}

impl MockAudioBackend {
    pub fn new() -> Self {
        Self {
            info: DeviceInfo { sample_rate: 48000, buffer_size: 256, channels: 2, device_name: Some("mock-device".to_string()) },
            render: Arc::new(Mutex::new(None)),
            frames: AtomicU64::new(0),
            diagnostics: None,
        }
    }
}

impl Default for MockAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for MockAudioBackend {
    fn start(&mut self, render: RenderFn) -> Result<(), BackendError> {
    let mut g = self.render.lock().unwrap();
    *g = Some(render);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), BackendError> {
    let mut g = self.render.lock().unwrap();
    *g = None;
        Ok(())
    }

    fn sample_rate(&self) -> u32 { self.info.sample_rate }
    fn buffer_size(&self) -> usize { self.info.buffer_size }
    fn channels(&self) -> u16 { self.info.channels }
    fn frames_since_start(&self) -> u64 { self.frames.load(Ordering::Relaxed) }
    fn set_diagnostics_callback(&mut self, cb: Option<DiagnosticsCb>) { self.diagnostics = cb; }

    fn as_device_info_provider(&self) -> Option<&dyn DeviceInfoProvider> {
        Some(self)
    }
}

impl DeviceInfoProvider for MockAudioBackend {
    fn get_device_name(&self) -> Option<&str> {
        self.info.device_name.as_deref()
    }
}
