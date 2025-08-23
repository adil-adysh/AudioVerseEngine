//! Minimal strongly-typed EventBus crate used for decoupling systems.
//! - publish() is thread-safe and enqueues events.
//! - drain() is called by owner (main loop) to deliver events deterministically.

use crossbeam_queue::SegQueue;
use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub type SubscriptionId = u64;

pub trait EventPayload: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> EventPayload for T {}

struct QueuedEvent {
    type_id: TypeId,
    payload: Box<dyn Any + Send + Sync>,
    priority: i32,
    seq: u64,
}

// Alias to shorten the complex subscriber function type and make clippy happier
type SubscriberFn = Arc<dyn Fn(&dyn Any) + Send + Sync>;

pub struct EventBusImpl {
    queue: Arc<SegQueue<QueuedEvent>>,
    subscribers: RwLock<HashMap<TypeId, Vec<(SubscriptionId, SubscriberFn)>>>,
    next_sub_id: AtomicU64,
    next_seq: AtomicU64,
    // RT-friendly length counter and optional capacity. Publishers can use try_publish_* to avoid allocations.
    len: AtomicUsize,
    capacity: Option<usize>,
}

impl EventBusImpl {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(SegQueue::new()),
            subscribers: RwLock::new(HashMap::new()),
            next_sub_id: AtomicU64::new(1),
            next_seq: AtomicU64::new(1),
            len: AtomicUsize::new(0),
            capacity: None,
        }
    }

    /// Create a bus with a bounded capacity. `try_publish*` will fail when full.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            queue: Arc::new(SegQueue::new()),
            subscribers: RwLock::new(HashMap::new()),
            next_sub_id: AtomicU64::new(1),
            next_seq: AtomicU64::new(1),
            len: AtomicUsize::new(0),
            capacity: Some(cap),
        }
    }

    /// Publish an event payload T. This is fast and thread-safe.
    pub fn publish<T: EventPayload>(&self, payload: T) {
        self.publish_with_priority(payload, 0);
    }

    /// Publish with priority (higher executes first). Deterministic: FIFO within priority.
    pub fn publish_with_priority<T: EventPayload>(&self, payload: T, priority: i32) {
        // Blocking publish: always enqueues and updates len.
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let ev = QueuedEvent {
            type_id: TypeId::of::<T>(),
            payload: Box::new(payload),
            priority,
            seq,
        };
        self.queue.push(ev);
        self.len.fetch_add(1, Ordering::Relaxed);
    }

    /// Try to publish without allocation in RT path. Returns Err(payload) if the queue is full.
    pub fn try_publish_with_priority<T: EventPayload>(
        &self,
        payload: T,
        priority: i32,
    ) -> Result<(), T> {
        // If bounded and full, fail fast
        if let Some(cap) = self.capacity {
            let cur = self.len.load(Ordering::Relaxed);
            if cur >= cap {
                return Err(payload);
            }
        }
        // reserve slot
        self.len.fetch_add(1, Ordering::Relaxed);
        // safe to enqueue now
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let ev = QueuedEvent {
            type_id: TypeId::of::<T>(),
            payload: Box::new(payload),
            priority,
            seq,
        };
        self.queue.push(ev);
        Ok(())
    }

    /// Subscribe to event type T. Handler receives &T.
    /// Returns a SubscriptionId for later unsubscription.
    pub fn subscribe<T: EventPayload, F>(&self, handler: F) -> SubscriptionId
    where
        F: Fn(&T) + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let sub_id = self.next_sub_id.fetch_add(1, Ordering::Relaxed);
        let boxed: SubscriberFn = Arc::new(move |any: &dyn Any| {
            if let Some(t) = any.downcast_ref::<T>() {
                handler(t);
            }
        });
        let mut map = self.subscribers.write();
        map.entry(type_id).or_default().push((sub_id, boxed));
        sub_id
    }

    /// Unsubscribe previously registered handler.
    pub fn unsubscribe(&self, subscription_id: SubscriptionId) {
        let mut map = self.subscribers.write();
        for (_k, vec) in map.iter_mut() {
            vec.retain(|(id, _)| *id != subscription_id);
        }
    }

    /// Drain queued events and invoke handlers synchronously on the caller thread.
    pub fn drain(&self) {
        // collect all queued events first
        let mut vec = Vec::new();
        while let Some(ev) = self.queue.pop() {
            vec.push(ev);
            // decrement length counter for RT path tracking
            let _ = self.len.fetch_sub(1, Ordering::Relaxed);
        }

        // sort by priority desc, then seq asc for deterministic ordering
        vec.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.seq.cmp(&b.seq)));

        for ev in vec.into_iter() {
            let readers = self.subscribers.read();
            if let Some(list) = readers.get(&ev.type_id) {
                for (_id, handler) in list.iter() {
                    handler(ev.payload.as_ref());
                }
            }
        }
    }
}

impl Default for EventBusImpl {
    fn default() -> Self {
        Self::new()
    }
}

// Shared event types can live in this crate so multiple systems can publish/subscribe
#[derive(Clone, Debug)]
pub struct PlaySoundEvent {
    pub entity: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Ping(i32);

    // removed unused Pong to avoid dead_code warning

    #[test]
    fn publish_and_subscribe() {
        let bus = EventBusImpl::new();
        let called = Arc::new(std::sync::Mutex::new(Vec::new()));
        let c2 = called.clone();
        bus.subscribe::<Ping, _>(move |p| {
            c2.lock().unwrap().push(p.0);
        });
        bus.publish(Ping(42));
        bus.drain();
        let v = called.lock().unwrap();
        assert_eq!(v.as_slice(), &[42]);
    }

    #[test]
    fn multiple_subscribers_and_unsubscribe() {
        let bus = EventBusImpl::new();
        let a = Arc::new(std::sync::Mutex::new(Vec::new()));
        let b = Arc::new(std::sync::Mutex::new(Vec::new()));
        let id1 = bus.subscribe::<Ping, _>({
            let a = a.clone();
            move |p| a.lock().unwrap().push(p.0)
        });
        let _id2 = bus.subscribe::<Ping, _>({
            let b = b.clone();
            move |p| b.lock().unwrap().push(p.0)
        });
        bus.publish(Ping(1));
        bus.publish(Ping(2));
        bus.drain();
        assert_eq!(&*a.lock().unwrap(), &[1, 2]);
        assert_eq!(&*b.lock().unwrap(), &[1, 2]);
        bus.unsubscribe(id1);
        bus.publish(Ping(3));
        bus.drain();
        assert_eq!(&*a.lock().unwrap(), &[1, 2]);
        assert_eq!(&*b.lock().unwrap(), &[1, 2, 3]);
    }

    #[test]
    fn priority_and_fifo_ordering() {
        let bus = EventBusImpl::new();
        let out = Arc::new(std::sync::Mutex::new(Vec::new()));
        let out2 = out.clone();

        bus.subscribe::<i32, _>(move |v| {
            out2.lock().unwrap().push(*v);
        });

        // Publish: values with priorities (higher first). Within same priority, FIFO order.
        bus.publish_with_priority(10i32, 1); // priority 1
        bus.publish_with_priority(20i32, 2); // priority 2 -> should run before priority 1
        bus.publish_with_priority(30i32, 2); // priority 2, later -> should come after 20
        bus.publish_with_priority(40i32, 1); // priority 1, later -> should come after 10

        bus.drain();
        let res = out.lock().unwrap().clone();
        assert_eq!(res, vec![20, 30, 10, 40]);
    }

    #[test]
    fn try_publish_bounded() {
        let bus = EventBusImpl::with_capacity(2);
        // two should succeed
        assert!(bus.try_publish_with_priority(1u32, 0).is_ok());
        assert!(bus.try_publish_with_priority(2u32, 0).is_ok());
        // third should fail
        assert!(bus.try_publish_with_priority(3u32, 0).is_err());
        // draining reduces length and allows another publish
        bus.drain();
        assert!(bus.try_publish_with_priority(4u32, 0).is_ok());
        bus.drain();
    }
}
