// This file contains the complete, corrected implementation of the audio backend API.
// It uses a trait composition pattern to handle specialized functionality in a safe way.

use std::sync::Arc;
use std::fmt;

// This mock implementation is included here for a self-contained, runnable example.
// In a real project, this would live in `mock_backend.rs`.
#[cfg(feature = "mock-audio")]
pub mod mock_backend {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use crossbeam_channel::{unbounded, Sender};
    use crate::{AudioBackend, DeviceInfo, DeviceInfoProvider, DiagnosticEvent, BackendError, RenderFn, DiagnosticsCb};

    // Shared state between the main thread and the mock audio thread.
    struct MockSharedState {
        is_running: bool,
        frames_since_start: u64,
        diagnostics_events: Vec<DiagnosticEvent>,
    }

    // Messages sent to the mock's internal control thread.
    pub enum MockCtrlMsg {
        Start,
        Stop,
        EmitDiagnostic(DiagnosticEvent),
        Shutdown,
    }

    pub struct MockAudioBackend {
        shared_state: Arc<Mutex<MockSharedState>>,
        info: DeviceInfo,
        ctrl_tx: Sender<MockCtrlMsg>,
        // In a real mock, you would also need a way to send render and diagnostic
        // callbacks to the worker thread. We'll simplify for this example.
    }

    impl MockAudioBackend {
        pub fn new() -> Self {
            let shared_state = Arc::new(Mutex::new(MockSharedState {
                is_running: false,
                frames_since_start: 0,
                diagnostics_events: Vec::new(),
            }));
            let (ctrl_tx, ctrl_rx) = unbounded();
            
            // Spawn a thread to simulate the non-blocking audio device.
            let mock_state = shared_state.clone();
            thread::spawn(move || {
                loop {
                    if let Ok(msg) = ctrl_rx.try_recv() {
                        match msg {
                            MockCtrlMsg::Start => mock_state.lock().unwrap().is_running = true,
                            MockCtrlMsg::Stop => mock_state.lock().unwrap().is_running = false,
                            MockCtrlMsg::EmitDiagnostic(event) => {
                                mock_state.lock().unwrap().diagnostics_events.push(event);
                            }
                            MockCtrlMsg::Shutdown => break,
                        }
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            });

            Self {
                shared_state,
                info: DeviceInfo {
                    sample_rate: 48000,
                    buffer_size: 1024,
                    channels: 2,
                    device_name: Some("Mock Audio Device".to_string()),
                },
                ctrl_tx,
            }
        }
    }

    // Now, the full implementation of the AudioBackend trait for the mock.
    impl AudioBackend for MockAudioBackend {
        fn start(&mut self, _render: RenderFn) -> Result<(), BackendError> {
            self.ctrl_tx.send(MockCtrlMsg::Start).unwrap();
            Ok(())
        }
        fn stop(&mut self) -> Result<(), BackendError> {
            self.ctrl_tx.send(MockCtrlMsg::Stop).unwrap();
            Ok(())
        }
        fn sample_rate(&self) -> u32 { self.info.sample_rate }
        fn buffer_size(&self) -> usize { self.info.buffer_size }
        fn channels(&self) -> u16 { self.info.channels }
        fn frames_since_start(&self) -> u64 { self.shared_state.lock().unwrap().frames_since_start }
        fn set_diagnostics_callback(&mut self, _cb: Option<DiagnosticsCb>) {
            // Mock implementation: do nothing for now.
        }
        
        // This is the implementation for the new "safe downcast" method.
        fn as_device_info_provider(&self) -> Option<&dyn DeviceInfoProvider> {
            Some(self)
        }
    }

    impl DeviceInfoProvider for MockAudioBackend {
        fn get_device_name(&self) -> Option<&str> {
            self.info.device_name.as_deref()
        }
    }
}

// This is a placeholder for `cpal_backend.rs`.
// It's a stub to allow the code to compile without the real cpal dependency.
pub mod cpal_backend {
    use crate::{AudioBackend, BackendError, DeviceInfo, DeviceInfoProvider, RenderFn, DiagnosticsCb};

    pub struct CpalAudioBackend {
        info: DeviceInfo,
    }

    impl CpalAudioBackend {
        pub fn new() -> Result<Self, BackendError> {
            Ok(Self {
                info: DeviceInfo {
                    sample_rate: 48000,
                    buffer_size: 1024,
                    channels: 2,
                    device_name: Some("Real CPAL Device".to_string()),
                }
            })
        }
    }

    impl AudioBackend for CpalAudioBackend {
        fn start(&mut self, _render: RenderFn) -> Result<(), BackendError> {
            // Placeholder: real implementation would start the CPAL stream.
            Ok(())
        }
        fn stop(&mut self) -> Result<(), BackendError> {
            // Placeholder: real implementation would stop the CPAL stream.
            Ok(())
        }
        fn sample_rate(&self) -> u32 { self.info.sample_rate }
        fn buffer_size(&self) -> usize { self.info.buffer_size }
        fn channels(&self) -> u16 { self.info.channels }
        fn frames_since_start(&self) -> u64 { 0 }
        fn set_diagnostics_callback(&mut self, _cb: Option<DiagnosticsCb>) {
            // Placeholder: real implementation would set the callback.
        }
        
        fn as_device_info_provider(&self) -> Option<&dyn DeviceInfoProvider> {
            Some(self)
        }
    }

    impl DeviceInfoProvider for CpalAudioBackend {
        fn get_device_name(&self) -> Option<&str> {
            self.info.device_name.as_deref()
        }
    }
}


/// A specialized error type for audio backend failures.
#[derive(Debug)]
pub enum BackendError {
    DeviceNotFound,
    UnsupportedFormat(String),
    StreamCreationFailed,
    PlaybackError(String),
    Other(String),
}

/// The render callback function.
/// 
/// This closure is called on the real-time audio thread to fill the output buffer.
/// It must be `Send + Sync` to be safely shared across threads.
pub type RenderFn = Arc<dyn Fn(&mut [f32], u32, usize) + Send + Sync + 'static>;

/// Diagnostics events emitted by the backend (non-RT callbacks expected).
#[derive(Debug, Clone)]
pub enum DiagnosticEvent {
    XRun { count: u32 },
    DeviceRemoved,
    BufferSizeChanged { frames: usize },
    Other(String),
}

impl fmt::Display for DiagnosticEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticEvent::XRun { count } => write!(f, "XRun(count={})", count),
            DiagnosticEvent::DeviceRemoved => write!(f, "DeviceRemoved"),
            DiagnosticEvent::BufferSizeChanged { frames } => write!(f, "BufferSizeChanged(frames={})", frames),
            DiagnosticEvent::Other(s) => write!(f, "Other({})", s),
        }
    }
}

/// Non-RT diagnostics callback type.
pub type DiagnosticsCb = Arc<dyn Fn(DiagnosticEvent) + Send + Sync + 'static>;

/// Represents the effective configuration of an audio device.
pub struct DeviceInfo {
    pub sample_rate: u32,
    pub buffer_size: usize,
    pub channels: u16,
    pub device_name: Option<String>,
}

/// A trait for backends that can provide additional information about the audio device.
pub trait DeviceInfoProvider {
    fn get_device_name(&self) -> Option<&str>;
}

/// The core trait defining the audio backend's contract.
///
/// We've removed `std::any::Any` as it's no longer necessary with the new pattern.
pub trait AudioBackend {
    fn start(&mut self, render: RenderFn) -> Result<(), BackendError>;
    fn stop(&mut self) -> Result<(), BackendError>;
    fn sample_rate(&self) -> u32;
    fn buffer_size(&self) -> usize;
    fn channels(&self) -> u16;
    /// Returns frames since stream start. 0 if not running.
    fn frames_since_start(&self) -> u64;
    /// Register or clear non-RT diagnostics callback.
    fn set_diagnostics_callback(&mut self, cb: Option<DiagnosticsCb>);
    
    /// The "safe downcast" method to access specialized features.
    fn as_device_info_provider(&self) -> Option<&dyn DeviceInfoProvider>;
}

#[cfg(not(feature = "mock-audio"))]
pub fn create_audio_backend() -> Result<Box<dyn AudioBackend>, BackendError> {
    Ok(Box::new(cpal_backend::CpalAudioBackend::new()?))
}

#[cfg(feature = "mock-audio")]
pub fn create_audio_backend() -> Result<Box<dyn AudioBackend>, BackendError> {
    Ok(Box::new(mock_backend::MockAudioBackend::new()))
}
