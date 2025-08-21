// This file contains the complete, corrected implementation of the audio backend API.
// It uses a trait composition pattern to handle specialized functionality in a safe way.

use std::sync::Arc;
use std::fmt;

// The mock backend implementation lives in `src/mock_backend.rs`.
#[cfg(feature = "mock-audio")]
pub mod mock_backend;

// The real CPAL-backed implementation lives in `src/cpal_backend.rs`.
#[cfg(not(feature = "mock-audio"))]
pub mod cpal_backend;


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
    let backend = cpal_backend::CpalAudioBackend::new()?;
    // Log backend info for diagnostics
    eprintln!("create_audio_backend: using CPAL backend -> sample_rate={} buffer_size={} channels={} device_name={}",
        backend.sample_rate(), backend.buffer_size(), backend.channels(), backend.as_device_info_provider().and_then(|d| d.get_device_name().map(|s| s.to_string())).unwrap_or_else(|| "<unknown>".to_string())
    );
    Ok(Box::new(backend))
}

/// Runtime helper to determine if the `mock-audio` feature was enabled at
/// compile time for this crate. Call from dependent crates/tests to confirm
/// which backend variant was compiled.
pub fn is_mock_backend_enabled() -> bool {
    cfg!(feature = "mock-audio")
}

#[cfg(feature = "mock-audio")]
pub fn create_audio_backend() -> Result<Box<dyn AudioBackend>, BackendError> {
    let backend = mock_backend::MockAudioBackend::new();
    eprintln!("create_audio_backend: using MOCK backend -> sample_rate={} buffer_size={} channels={} device_name={}",
        backend.sample_rate(), backend.buffer_size(), backend.channels(), backend.as_device_info_provider().and_then(|d| d.get_device_name().map(|s| s.to_string())).unwrap_or_else(|| "<unknown>".to_string())
    );
    Ok(Box::new(backend))
}
