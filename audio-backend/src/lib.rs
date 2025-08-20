pub mod cpal_backend;
pub mod mock_backend;

use std::sync::Arc;
use std::fmt;

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
}

/// The core trait defining the audio backend's contract.
/// 
/// All backend implementations must adhere to this interface.
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
}

#[cfg(not(feature = "mock-audio"))]
pub fn create_audio_backend() -> Result<Box<dyn AudioBackend>, BackendError> {
    Ok(Box::new(cpal_backend::CpalAudioBackend::new()?))
}
#[cfg(feature = "mock-audio")]
pub fn create_audio_backend() -> Result<Box<dyn AudioBackend>, BackendError> {
    Ok(Box::new(mock_backend::MockAudioBackend::new()))
}
