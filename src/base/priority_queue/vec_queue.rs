//! Priority queue implemented as a simple sorted vector
//! O(n) insertion, O(1) pop_first, O(n) removal

use crate::base::sim_time::SimTime;
use super::priority_queue::Node;
use super::priority_queue_trait::{EventQueue, EventPriorities};
use std::fmt::Debug;
use std::marker::PhantomData;

/// Priority queue implemented as a plain vector with descending key order, so smallest 
/// (time, priority, event_id) is in the last position.
/// Insertion: O(n), scanning from right to left to find insertion point
/// Pop first: O(1), the minimum is always the last element.
/// Removal: O(n), linear scan to find matching event_id.
pub struct VecQueue<T: Debug + Copy, P: EventPriorities<T>> {
    items: Vec<Node<T>>,
    _priorities: PhantomData<P>,
}

// Implementing methods for VecQueue that will be used as private helpers when
// implementing the EventQueue interface
impl<T: Debug + Copy, P: EventPriorities<T>> VecQueue<T, P> {

    pub fn with_capacity(capacity: usize) -> Self {
        VecQueue {
            items: Vec::with_capacity(capacity),
            _priorities: PhantomData,
        }
    }

    pub fn new() -> Self {
        VecQueue {
            items: Vec::new(),
            _priorities: PhantomData,
        }
    }

    /// Gets key (time, priority, event_id) from a node reference. The key tuple is used
    /// to tie-break events with the same time
    fn key(node: &Node<T>) -> (SimTime, u32, u64) {
        (node.time, node.priority, node.event_id)
    }
}

// Implementing EventQueue trait for VecQueue
impl<T: Debug + Copy, P: EventPriorities<T>> EventQueue<T, P> for VecQueue<T, P> {

    /// New trait just uses VecQueue constructor
    fn new() -> Self {
        VecQueue::new()
    }

    /// Scan from right to left and when you find key > new_key you insert
    fn insert(&mut self, event_id: u64, time: SimTime, event_type: T) -> Result<(), String> {

        // We check that time is not negative otherwise error
        if time < 0 {
            return Err("Event time cannot be negative".to_string());
        }

        // Create Node and get key of the node
        let priority = P::priority(&event_type);
        let node = Node { event_id, time, event_type, priority };
        let new_key = Self::key(&node);

        // From right to left and when you find key > new_key you insert there
        let mut pos = self.items.len();
        while pos > 0 && Self::key(&self.items[pos - 1]) < new_key {
            pos -= 1;
        }
        self.items.insert(pos, node);

        Ok(())
    }

    /// Just pop last elemnt
    fn pop_first(&mut self) -> Option<Node<T>> {
        self.items.pop()
    }

    /// The minimum is always the last element. O(1)
    fn peek_first(&self) -> Option<SimTime> {
        self.items.last().map(|node| node.time)
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn clear(&mut self) {
        self.items.clear();
    }

    /// Iterate over the vector until the matching event_id is found, then remove it. O(n)
    fn remove(&mut self, _time: SimTime, event_id: u64) -> bool {
        for idx in 0..self.items.len() {
            if self.items[idx].event_id == event_id {
                self.items.remove(idx);
                return true;
            }
        }
        false
    }
}

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
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();

        queue.insert(next_id(), 10, EventType::Arrival).unwrap();
        queue.insert(next_id(), 5, EventType::Departure).unwrap();
        queue.insert(next_id(), 15, EventType::ServiceComplete).unwrap();

        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(15));
        assert!(queue.pop_first().is_none());
    }

    #[test]
    fn test_peek_first() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();

        queue.insert(next_id(), 20, EventType::Arrival).unwrap();
        queue.insert(next_id(), 10, EventType::Departure).unwrap();

        assert_eq!(queue.peek_first(), Some(10));
        assert_eq!(queue.len(), 2);

        queue.pop_first();
        assert_eq!(queue.peek_first(), Some(20));
    }

    #[test]
    fn test_empty_queue() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert!(queue.pop_first().is_none());
        assert_eq!(queue.peek_first(), None);
    }

    #[test]
    fn test_negative_time_rejected() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();
        assert_eq!(
            queue.insert(next_id(), -1, EventType::Arrival),
            Err("Event time cannot be negative".to_string())
        );
    }

    #[test]
    fn test_multiple_events_same_time() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();

        queue.insert(next_id(), 10, EventType::Arrival).unwrap();
        queue.insert(next_id(), 10, EventType::Departure).unwrap();
        queue.insert(next_id(), 10, EventType::ServiceComplete).unwrap();
        queue.insert(next_id(), 5, EventType::Arrival).unwrap();

        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert!(queue.pop_first().is_none());
    }

    #[test]
    fn test_remove_event() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();

        let id1 = next_id();
        let id2 = next_id();
        let id3 = next_id();
        let id4 = next_id();
        let id5 = next_id();

        queue.insert(id1, 10, EventType::Arrival).unwrap();
        queue.insert(id2, 20, EventType::Arrival).unwrap();
        queue.insert(id3, 30, EventType::Arrival).unwrap();
        queue.insert(id4, 25, EventType::Arrival).unwrap();
        queue.insert(id5, 15, EventType::Arrival).unwrap();

        assert_eq!(queue.len(), 5);
        assert!(queue.remove(20, id2));
        assert_eq!(queue.len(), 4);
        assert!(queue.remove(10, id1));
        assert_eq!(queue.len(), 3);
        assert!(queue.remove(30, id3));
        assert_eq!(queue.len(), 2);

        assert_eq!(queue.pop_first().map(|n| n.time), Some(15));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(25));
        assert!(queue.pop_first().is_none());
    }

    #[test]
    fn test_remove_missing_event_returns_false() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();
        queue.insert(next_id(), 10, EventType::Arrival).unwrap();
        assert!(!queue.remove(10, 9999));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_remove_head() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();

        let handle_first = next_id();
        queue.insert(handle_first, 5, EventType::Arrival).unwrap();
        queue.insert(next_id(), 10, EventType::Arrival).unwrap();

        assert_eq!(queue.peek_first(), Some(5));
        assert!(queue.remove(5, handle_first));
        assert_eq!(queue.peek_first(), Some(10));
    }

    #[test]
    fn test_remove_tail() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();

        queue.insert(next_id(), 5, EventType::Arrival).unwrap();
        let handle_last = next_id();
        queue.insert(handle_last, 20, EventType::Arrival).unwrap();

        assert_eq!(queue.len(), 2);
        assert!(queue.remove(20, handle_last));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
    }

    #[test]
    fn test_clear() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();

        queue.insert(next_id(), 10, EventType::Arrival).unwrap();
        queue.insert(next_id(), 20, EventType::Arrival).unwrap();

        assert_eq!(queue.len(), 2);
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_insertion_order_preserved() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();

        queue.insert(next_id(), 30, EventType::Arrival).unwrap();
        queue.insert(next_id(), 10, EventType::Arrival).unwrap();
        queue.insert(next_id(), 20, EventType::Arrival).unwrap();
        queue.insert(next_id(), 15, EventType::Arrival).unwrap();

        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(15));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(20));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(30));
    }

    #[test]
    fn test_priority_tiebreak_same_time() {
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();

        queue.insert(next_id(), 10, EventType::ServiceComplete).unwrap();
        queue.insert(next_id(), 10, EventType::Departure).unwrap();
        queue.insert(next_id(), 10, EventType::Arrival).unwrap();

        let first = queue.pop_first().unwrap();
        assert_eq!(first.priority, 0);
        let second = queue.pop_first().unwrap();
        assert_eq!(second.priority, 1);
        let third = queue.pop_first().unwrap();
        assert_eq!(third.priority, 2);
    }

    #[test]
    fn test_sorted_order_maintained_after_mixed_ops() {
        // Regression check: insertions interleaved with removals must keep the vector
        // sorted so pop_first (a plain Vec::pop) still returns the true minimum.
        let mut queue: VecQueue<EventType, TestPriorities> = VecQueue::new();

        let id1 = next_id();
        queue.insert(id1, 50, EventType::Arrival).unwrap();
        queue.insert(next_id(), 10, EventType::Arrival).unwrap();
        let id3 = next_id();
        queue.insert(id3, 30, EventType::Arrival).unwrap();
        queue.insert(next_id(), 20, EventType::Arrival).unwrap();

        assert!(queue.remove(30, id3));
        queue.insert(next_id(), 5, EventType::Arrival).unwrap();
        assert!(queue.remove(50, id1));

        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(20));
        assert!(queue.pop_first().is_none());
    }
}
