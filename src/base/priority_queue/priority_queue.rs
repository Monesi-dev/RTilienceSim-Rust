use crate::base::sim_time::SimTime;
use super::priority_queue_trait::{EventQueue, EventPriorities};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait Event: Debug {
    fn get_time(&self) -> SimTime;
    fn set_time(&mut self, time: SimTime);
    fn get_id(&self) -> u64;
    fn set_id(&mut self, id: u64);
    fn doit(&mut self) -> (bool, Vec<Box<dyn Event>>);
    fn clone_box(&self) -> Box<dyn Event>;
}

#[derive(Debug)]
pub struct Node<T: Debug + Copy> {
    pub event_id: u64,
    pub time: SimTime,
    pub event_type: T,
    /// Cached result of `P::priority(&event_type)` at insertion time, used to tie-break
    /// nodes sharing the same `time` (lower value is popped first).
    pub priority: u32,
}

// Priority queue using BTreeMap: O(log n) for access and removal.
// Stores nodes with time, event_id, and event_type; actual event data is managed externally by the scheduler.
// Since multiple events can occur at the same time, we store a vector of nodes and pick
// the one with the lowest (priority, event_id) among them when popping.
#[derive(Debug)]
pub struct BTreeQueue<T: Debug + Copy, P: EventPriorities<T>> {
    items: BTreeMap<SimTime, Vec<Node<T>>>,
    total_count: usize,
    _priorities: PhantomData<P>,
}

impl<T: Debug + Copy, P: EventPriorities<T>> BTreeQueue<T, P> {
    pub fn new() -> Self {
        BTreeQueue {
            items: BTreeMap::new(),
            total_count: 0,
            _priorities: PhantomData,
        }
    }

    /// Insert a node (event_id + time + event_type) into the queue. O(log n)
    pub fn insert(&mut self, event_id: u64, time: SimTime, event_type: T) -> Result<(), String> {
        if time < 0 {
            return Err("Event time cannot be negative".to_string());
        }

        let priority = P::priority(&event_type);
        let node = Node { event_id, time, event_type, priority };
        let nodes = self.items.entry(time).or_insert_with(Vec::new);
        nodes.push(node);
        self.total_count += 1;
        Ok(())
    }

    /// Extract the first (minimum time) node. Among nodes sharing that time, the one
    /// with the lowest (priority, event_id) is picked. O(log n) + O(k) where k is the
    /// number of events tied at that time (typically small).
    pub fn pop_first(&mut self) -> Option<Node<T>> {
        let first_key = self.items.keys().next().copied()?;
        let nodes = self.items.get_mut(&first_key)?;

        if nodes.is_empty() {
            return None;
        }

        let mut min_idx = 0;
        for (idx, node) in nodes.iter().enumerate() {
            let min_node = &nodes[min_idx];
            if (node.priority, node.event_id) < (min_node.priority, min_node.event_id) {
                min_idx = idx;
            }
        }

        let node = nodes.remove(min_idx);
        self.total_count -= 1;

        if nodes.is_empty() {
            self.items.remove(&first_key);
        }

        Some(node)
    }

    pub fn peek_first(&self) -> Option<SimTime> {
        self.items.keys().next().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.total_count
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.total_count = 0;
    }

    /// Remove a specific node by event_id and time. O(log n)
    pub fn remove(&mut self, time: SimTime, event_id: u64) -> bool {
        let Some(nodes) = self.items.get_mut(&time) else {
            return false;
        };

        for (idx, node) in nodes.iter().enumerate() {
            if node.event_id == event_id {
                nodes.remove(idx);
                self.total_count -= 1;

                if nodes.is_empty() {
                    self.items.remove(&time);
                }
                return true;
            }
        }
        false
    }
}

impl<T: Debug + Copy, P: EventPriorities<T>> EventQueue<T, P> for BTreeQueue<T, P> {
    fn new() -> Self {
        BTreeQueue::new()
    }

    fn insert(&mut self, event_id: u64, time: SimTime, event_type: T) -> Result<(), String> {
        BTreeQueue::insert(self, event_id, time, event_type)
    }

    fn pop_first(&mut self) -> Option<Node<T>> {
        BTreeQueue::pop_first(self)
    }

    fn peek_first(&self) -> Option<SimTime> {
        BTreeQueue::peek_first(self)
    }

    fn is_empty(&self) -> bool {
        BTreeQueue::is_empty(self)
    }

    fn len(&self) -> usize {
        BTreeQueue::len(self)
    }

    fn clear(&mut self) {
        BTreeQueue::clear(self)
    }

    fn remove(&mut self, time: SimTime, event_id: u64) -> bool {
        BTreeQueue::remove(self, time, event_id)
    }
}

// Unit tests for BTreeQueue
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug, Clone, Copy)]
    enum EventType {
        Arrival,
        Departure,
        ServiceComplete,
    }

    #[derive(Debug, Clone, Copy)]
    struct TestPriorities;

    impl EventPriorities<EventType> for TestPriorities {
        fn priority(event_type: &EventType) -> u32 {
            match event_type {
                EventType::Arrival => 0,
                EventType::Departure => 1,
                EventType::ServiceComplete => 2,
            }
        }
    }

    fn next_id() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    #[test]
    fn test_insert_and_pop() {
        let mut queue: BTreeQueue<EventType, TestPriorities> = BTreeQueue::new();

        queue.insert(next_id(), 10, EventType::Arrival).unwrap();
        queue.insert(next_id(), 5, EventType::Departure).unwrap();
        queue.insert(next_id(), 15, EventType::ServiceComplete).unwrap();

        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(15));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_peek_first() {
        let mut queue: BTreeQueue<EventType, TestPriorities> = BTreeQueue::new();

        queue.insert(next_id(), 20, EventType::Arrival).unwrap();
        queue.insert(next_id(), 10, EventType::Departure).unwrap();

        assert_eq!(queue.peek_first(), Some(10));
        assert_eq!(queue.len(), 2);

        queue.pop_first();
        assert_eq!(queue.peek_first(), Some(20));
    }

    #[test]
    fn test_empty_queue() {
        let mut queue: BTreeQueue<EventType, TestPriorities> = BTreeQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert!(queue.pop_first().is_none());
    }

    #[test]
    fn test_negative_time_rejected() {
        let mut queue: BTreeQueue<EventType, TestPriorities> = BTreeQueue::new();
        assert_eq!(
            queue.insert(next_id(), -1, EventType::Arrival),
            Err("Event time cannot be negative".to_string())
        );
    }

    #[test]
    fn test_multiple_events_same_time() {
        let mut queue: BTreeQueue<EventType, TestPriorities> = BTreeQueue::new();

        queue.insert(next_id(), 10, EventType::Arrival).unwrap();
        queue.insert(next_id(), 10, EventType::Departure).unwrap();
        queue.insert(next_id(), 10, EventType::ServiceComplete).unwrap();
        queue.insert(next_id(), 5, EventType::Arrival).unwrap();

        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_remove_event() {
        let mut queue: BTreeQueue<EventType, TestPriorities> = BTreeQueue::new();

        let id1 = next_id();
        let id2 = next_id();
        let id3 = next_id();

        queue.insert(id1, 10, EventType::Arrival).unwrap();
        queue.insert(id2, 10, EventType::Departure).unwrap();
        queue.insert(id3, 20, EventType::ServiceComplete).unwrap();

        assert_eq!(queue.len(), 3);

        let removed = queue.remove(10, id1);
        assert!(removed);
        assert_eq!(queue.len(), 2);

        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(20));
        assert!(queue.is_empty());
    }
}
