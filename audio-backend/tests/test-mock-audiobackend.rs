// This is a test-only mock implementation of the AudioBackend trait.
// It uses channels to simulate the asynchronous behavior of a real-time backend.
// This mock allows the test code to verify that the core logic works as expected.
// Self-contained Rust test file for audio-backend.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crossbeam_channel::{unbounded, Sender};
use audio_backend::{AudioBackend, DiagnosticEvent, RenderFn, DiagnosticsCb, BackendError, DeviceInfoProvider};

// Mock implementation of the AudioBackend.
// It holds a shared state that is accessible from both the test thread and the mock worker thread.
pub struct MockAudioBackend {
    shared_state: Arc<Mutex<MockSharedState>>,
    ctrl_tx: Sender<MockCtrlMsg>,
}

// Data shared between the test thread and the mock worker.
struct MockSharedState {
    is_running: bool,
    frames_since_start: u64,
    diagnostics_events: Vec<DiagnosticEvent>,
}

// Messages sent from the test thread to the mock worker.
enum MockCtrlMsg {
    Start,
    Stop,
    EmitDiagnostic(DiagnosticEvent),
    Shutdown,
    SetRender(Option<RenderFn>),
    SetDiagnostics(Option<DiagnosticsCb>),
}

// A simple mock for `create_audio_backend()` for tests only.
#[cfg(test)]
pub fn create_audio_backend() -> Result<Box<dyn AudioBackend>, BackendError> {
    Ok(Box::new(MockAudioBackend::new()))
}

impl MockAudioBackend {
    pub fn new() -> Self {
        let shared_state = Arc::new(Mutex::new(MockSharedState {
            is_running: false,
            frames_since_start: 0,
            diagnostics_events: Vec::new(),
        }));

        let (ctrl_tx, ctrl_rx) = unbounded();

        let mock_state = shared_state.clone();
        thread::spawn(move || {
            let mut render_fn: Option<RenderFn> = None;
            let mut diag_cb: Option<DiagnosticsCb> = None;
            let mut buf = [0.0f32; 1024];

            loop {
                // Try to receive a control message without blocking.
                if let Ok(msg) = ctrl_rx.try_recv() {
                    match msg {
                        MockCtrlMsg::Start => {
                            mock_state.lock().unwrap().is_running = true;
                        }
                        MockCtrlMsg::Stop => {
                            mock_state.lock().unwrap().is_running = false;
                        }
                        MockCtrlMsg::EmitDiagnostic(event) => {
                            if let Some(cb) = &diag_cb {
                                cb(event);
                            }
                        }
                        MockCtrlMsg::Shutdown => break,
                        MockCtrlMsg::SetRender(func) => render_fn = func,
                        MockCtrlMsg::SetDiagnostics(cb) => diag_cb = cb,
                    }
                }

                if mock_state.lock().unwrap().is_running {
                    if let Some(render) = &render_fn {
                        // Simulate the audio callback by calling the render function.
                        // Catch panics from the render closure so the mock worker stays alive
                        // and follows the real-backend contract: on panic, output silence.
                        let sample_rate = 48000;
                        let frames = buf.len() / 2; // Assuming 2 channels
                        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            render(&mut buf, sample_rate, frames);
                        }));
                        if res.is_err() {
                            // Clear buffer on panic (output silence) and continue.
                            buf.iter_mut().for_each(|s| *s = 0.0);
                        }

                        mock_state.lock().unwrap().frames_since_start += frames as u64;
                    }
                }
                
                // Simulate a time interval to prevent the loop from spinning too fast.
                thread::sleep(Duration::from_millis(5));
            }
        });

        Self {
            shared_state,
            ctrl_tx,
        }
    }
}

impl AudioBackend for MockAudioBackend {
    fn start(&mut self, render: crate::RenderFn) -> Result<(), crate::BackendError> {
        self.ctrl_tx.send(MockCtrlMsg::SetRender(Some(render))).unwrap();
        self.ctrl_tx.send(MockCtrlMsg::Start).unwrap();
        Ok(())
    }

    fn stop(&mut self) -> Result<(), crate::BackendError> {
        self.ctrl_tx.send(MockCtrlMsg::Stop).unwrap();
        self.ctrl_tx.send(MockCtrlMsg::SetRender(None)).unwrap();
        Ok(())
    }

    fn sample_rate(&self) -> u32 { 48000 }
    fn buffer_size(&self) -> usize { 1024 }
    fn channels(&self) -> u16 { 2 }
    fn frames_since_start(&self) -> u64 { self.shared_state.lock().unwrap().frames_since_start }
    fn set_diagnostics_callback(&mut self, cb: Option<crate::DiagnosticsCb>) {
        self.ctrl_tx.send(MockCtrlMsg::SetDiagnostics(cb)).unwrap();
    }
    
    // This is the new method that was missing.
    fn as_device_info_provider(&self) -> Option<&dyn DeviceInfoProvider> {
        Some(self)
    }
}

// We also need to implement the DeviceInfoProvider trait for our mock backend.
impl DeviceInfoProvider for MockAudioBackend {
    fn get_device_name(&self) -> Option<&str> {
        Some("Mock Audio Device")
    }
}


// A simple render function for testing
fn test_render_fn(data: &mut [f32], _sample_rate: u32, _frames: usize) {
    // Fill the buffer with a simple sine wave or just a non-zero value
    data.iter_mut().for_each(|s| *s = 0.5);
}

// A render function that panics
fn panic_render_fn(_data: &mut [f32], _sample_rate: u32, _frames: usize) {
    panic!("Test panic in render function");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread::sleep;
    use std::time::Duration;
    use crate::DiagnosticEvent;

    // A simple mock of a diagnostics callback.
    // It stores received events in a vector to be checked later.
    fn create_mock_diag_cb() -> (crate::DiagnosticsCb, Arc<Mutex<Vec<DiagnosticEvent>>>) {
        let received_events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = received_events.clone();
        let callback = Arc::new(move |event| {
            events_clone.lock().unwrap().push(event);
        });
        (callback, received_events)
    }

    // A small helper to wait for the mock backend to process messages.
    fn wait_for_mock_processing() {
        sleep(Duration::from_millis(100));
    }

    #[test]
    fn test_create_and_initial_state() {
        let backend = create_audio_backend().unwrap();
        assert_eq!(backend.frames_since_start(), 0);
        assert!(backend.sample_rate() > 0);
        assert!(backend.channels() > 0);
        
        // Test the new method
        let provider = backend.as_device_info_provider().unwrap();
        assert_eq!(provider.get_device_name(), Some("Mock Audio Device"));
    }

    #[test]
    fn test_start_and_stop() {
    let mut backend = MockAudioBackend::new();
        let render_fn = Arc::new(test_render_fn);

        // Start the stream and wait for frames to accumulate.
        backend.start(render_fn.clone()).unwrap();
        wait_for_mock_processing();
        assert!(backend.frames_since_start() > 0);

        // Stop the stream and wait for it to stop.
        backend.stop().unwrap();
        let initial_frames = backend.frames_since_start();
        wait_for_mock_processing();

        // Verify that frames have stopped accumulating.
        assert_eq!(backend.frames_since_start(), initial_frames);
    }

    #[test]
    fn test_frames_counter() {
        let mut backend = create_audio_backend().unwrap();
        let render_fn = Arc::new(test_render_fn);

        backend.start(render_fn).unwrap();
        let frames_1 = backend.frames_since_start();
        wait_for_mock_processing();
        let frames_2 = backend.frames_since_start();
        wait_for_mock_processing();
        let frames_3 = backend.frames_since_start();

        assert!(frames_2 > frames_1);
        assert!(frames_3 > frames_2);
    }

    #[test]
    fn test_diagnostics_callback() {
        // Use the concrete mock backend so tests can access internal channels.
        let mut backend = MockAudioBackend::new();
        let (cb, received_events) = create_mock_diag_cb();

        // Set the callback and send a diagnostic event via the mock's internal channel.
        backend.set_diagnostics_callback(Some(cb));

    // We constructed a concrete `MockAudioBackend` above, so use it directly.
    {
        let mock_ref: &MockAudioBackend = &backend;

        // Emit an XRun diagnostic and wait for it to be processed.
        mock_ref.ctrl_tx.send(MockCtrlMsg::EmitDiagnostic(DiagnosticEvent::XRun { count: 1 })).unwrap();
        wait_for_mock_processing();

        // Verify the event was received.
        let events = received_events.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            DiagnosticEvent::XRun { count } => assert_eq!(*count, 1),
            _ => panic!("Unexpected event type"),
        }
    }

    // Clear the callback and verify no more events are received.
    backend.set_diagnostics_callback(None);
    // Re-borrow after clearing the callback.
    {
        let mock_ref: &MockAudioBackend = &backend;
        mock_ref.ctrl_tx.send(MockCtrlMsg::EmitDiagnostic(DiagnosticEvent::DeviceRemoved)).unwrap();
        wait_for_mock_processing();
    }
    let events = received_events.lock().unwrap();
    assert_eq!(events.len(), 1); // No new events should have been added.
    }

    #[test]
    fn test_render_fn_panic_is_caught() {
        let mut backend = create_audio_backend().unwrap();
        let render_fn = Arc::new(panic_render_fn);

        // This test assumes that the `cpal_backend` implementation catches panics.
        // We'll simulate this by starting with the panic-prone function and observing behavior.
        // In the real implementation, the panic is caught and the buffer is cleared.
        // The mock can't fully simulate a panic, but we'd test this on the real backend.
        // We can at least ensure the mock's state doesn't crash on this input.
        backend.start(render_fn).unwrap();
        wait_for_mock_processing();
        
        // The frames counter should still be increasing, showing the stream didn't crash.
        let frames_1 = backend.frames_since_start();
        wait_for_mock_processing();
        let frames_2 = backend.frames_since_start();
        assert!(frames_2 > frames_1);
    }
}
