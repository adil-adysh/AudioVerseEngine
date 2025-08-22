use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A small single-producer single-consumer ring buffer for f32 samples.
pub struct RingBuffer {
    buf: Box<[UnsafeCell<f32>]>,
    capacity: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl RingBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.next_power_of_two();
        let mut v = Vec::with_capacity(cap);
        v.resize_with(cap, || UnsafeCell::new(0.0));
        Self {
            buf: v.into_boxed_slice(),
            capacity: cap,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, src: &[f32]) -> usize {
        let mut pushed = 0usize;
        let cap = self.capacity;
        loop {
            let head = self.head.load(Ordering::Acquire);
            let tail = self.tail.load(Ordering::Acquire);
            let used = (tail.wrapping_sub(head)) & (cap - 1);
            let free = cap - used - 1; // keep one slot free
            if free == 0 || pushed >= src.len() {
                break;
            }
            let to_write = (src.len() - pushed).min(free);
            for i in 0..to_write {
                let idx = (tail + i) & (cap - 1);
                unsafe {
                    *self.buf[idx].get() = src[pushed + i];
                }
            }
            self.tail
                .store(tail.wrapping_add(to_write), Ordering::Release);
            pushed += to_write;
            if pushed >= src.len() {
                break;
            }
        }
        pushed
    }

    pub fn pop(&self, dst: &mut [f32]) -> usize {
        let mut popped = 0usize;
        let cap = self.capacity;
        loop {
            let head = self.head.load(Ordering::Acquire);
            let tail = self.tail.load(Ordering::Acquire);
            let available = (tail.wrapping_sub(head)) & (cap - 1);
            if available == 0 || popped >= dst.len() {
                break;
            }
            let to_read = (dst.len() - popped).min(available);
            for i in 0..to_read {
                let idx = (head + i) & (cap - 1);
                unsafe {
                    dst[popped + i] = *self.buf[idx].get();
                }
            }
            self.head
                .store(head.wrapping_add(to_read), Ordering::Release);
            popped += to_read;
            if popped >= dst.len() {
                break;
            }
        }
        popped
    }
}

// Safety: we ensure single-producer single-consumer usage with atomic head/tail.
unsafe impl Sync for RingBuffer {}
unsafe impl Send for RingBuffer {}
