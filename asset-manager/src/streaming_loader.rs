use crate::Error;

#[cfg(feature = "streaming")]
use ringbuf::{HeapRb, traits::Producer};
#[cfg(feature = "streaming")]
use std::thread;

#[cfg(feature = "streaming")]
pub struct StreamingAsset {
    consumer: ringbuf::HeapCons<f32>,
    _handle: thread::JoinHandle<()>,
}

#[cfg(feature = "streaming")]
impl StreamingAsset {
    pub fn open(path: &str) -> Result<StreamingAsset, String> {
        let rb = HeapRb::<f32>::new(32 * 1024);
        let (mut prod, cons) = rb.split();
        let path_str = path.to_string();
        let handle = thread::spawn(move || {
            // for simplicity, re-use logic from asset_manager's streaming implementation
            if let Ok(data) = std::fs::read(&path_str) {
                // naive raw push if file appears to be .pcm format
                let mut i = 8usize;
                while i + 4 <= data.len() {
                    let b = [data[i], data[i + 1], data[i + 2], data[i + 3]];
                    let _ = prod.push(f32::from_le_bytes(b));
                    i += 4;
                }
            }
        });

        Ok(StreamingAsset { consumer: cons, _handle: handle })
    }

    pub fn read(&mut self, out: &mut [f32]) -> usize {
        self.consumer.pop_slice(out)
    }
}
