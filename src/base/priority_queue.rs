use crate::base::sim_time::SimTime;
use std::collections::BTreeMap;
use std::fmt::Debug;

pub trait Event: Debug {
    fn get_time(&self) -> SimTime;
    fn set_time(&mut self, time: SimTime);
    fn get_id(&self) -> u64;
    fn set_id(&mut self, id: u64);
    fn doit(&mut self) -> (bool, Vec<Box<dyn Event>>);
    fn clone_box(&self) -> Box<dyn Event>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    pub event_id: u64,
    pub time: SimTime,
}

// Priority queue using BTreeMap: O(log n) for access and removal.
// Stores only time and event_id; actual event data is managed externally by the scheduler.
// Since multiple events can occur at the same time, we store a vector of nodes.
#[derive(Debug)]
pub struct PriorityQueue {
    items: BTreeMap<SimTime, Vec<Node>>,
    total_count: usize,
}

impl PriorityQueue {
    pub fn new() -> Self {
        PriorityQueue {
            items: BTreeMap::new(),
            total_count: 0,
        }
    }

    /// Insert a node (event_id + time) into the queue. O(log n)
    pub fn insert(&mut self, event_id: u64, time: SimTime) -> Result<(), String> {
        if time < 0 {
            return Err("Event time cannot be negative".to_string());
        }

        let node = Node { event_id, time };
        let nodes = self.items.entry(time).or_insert_with(Vec::new);
        nodes.push(node);
        self.total_count += 1;
        Ok(())
    }

    /// Extract the first (minimum time) node. O(log n)
    pub fn pop_first(&mut self) -> Option<Node> {
        let first_key = self.items.keys().next().copied()?;
        let nodes = self.items.get_mut(&first_key)?;

        if nodes.is_empty() {
            return None;
        }

        let node = nodes.remove(0);
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

// Unit tests for PriorityQueue
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn next_id() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    #[test]
    fn test_insert_and_pop() {
        let mut queue = PriorityQueue::new();

        queue.insert(next_id(), 10).unwrap();
        queue.insert(next_id(), 5).unwrap();
        queue.insert(next_id(), 15).unwrap();

        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(15));
        assert_eq!(queue.pop_first(), None);
    }

    #[test]
    fn test_peek_first() {
        let mut queue = PriorityQueue::new();

        queue.insert(next_id(), 20).unwrap();
        queue.insert(next_id(), 10).unwrap();

        assert_eq!(queue.peek_first(), Some(10));
        assert_eq!(queue.len(), 2);

        queue.pop_first();
        assert_eq!(queue.peek_first(), Some(20));
    }

    #[test]
    fn test_empty_queue() {
        let mut queue = PriorityQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.pop_first(), None);
    }

    #[test]
    fn test_negative_time_rejected() {
        let mut queue = PriorityQueue::new();
        assert_eq!(
            queue.insert(next_id(), -1),
            Err("Event time cannot be negative".to_string())
        );
    }

    #[test]
    fn test_multiple_events_same_time() {
        let mut queue = PriorityQueue::new();

        queue.insert(next_id(), 10).unwrap();
        queue.insert(next_id(), 10).unwrap();
        queue.insert(next_id(), 10).unwrap();
        queue.insert(next_id(), 5).unwrap();

        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert!(queue.pop_first().is_none());
    }

    #[test]
    fn test_remove_event() {
        let mut queue = PriorityQueue::new();

        let id1 = next_id();
        let id2 = next_id();
        let id3 = next_id();

        queue.insert(id1, 10).unwrap();
        queue.insert(id2, 10).unwrap();
        queue.insert(id3, 20).unwrap();

        assert_eq!(queue.len(), 3);

        let removed = queue.remove(10, id1);
        assert!(removed);
        assert_eq!(queue.len(), 2);

        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(20));
        assert_eq!(queue.pop_first(), None);
    }
}
