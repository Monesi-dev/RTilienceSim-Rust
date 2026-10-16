//! Priority queue implemented as an indexed binary heap with HashMap indexing
//! O(log n) insertion, O(log n) pop_first, O(log n) removal

use crate::base::sim_time::SimTime;
use super::priority_queue::Node;
use super::priority_queue_trait::{EventQueue, EventPriorities};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

type HeapIdx = usize;

/// Priority queue implemented as an indexed binary heap.
/// You keep values in a binary tree where each node is smaller than its two children,
/// smallest node will be at the top. The tree is implemented as a vector (node at index
/// idx has as children the nodes at indices 2*idx+1 and 2*idx+2).
/// A hash map maps event_id -> heap index in O(1) so removal can be done in O(log n)
/// instead of O(n).
/// Insertion: O(log n)
/// Pop first: O(log n)
/// Removal: O(log n)
pub struct BinaryHeapQueue<T: Debug + Copy, P: EventPriorities<T>> {
    heap: Vec<Node<T>>,
    index: HashMap<u64, HeapIdx>,
    _priorities: PhantomData<P>,
}

// Implementing methods for BinaryHeapQueue that will be used as private helpers when
// implementing the EventQueue interface
impl<T: Debug + Copy, P: EventPriorities<T>> BinaryHeapQueue<T, P> {

    pub fn with_capacity(capacity: usize) -> Self {
        BinaryHeapQueue {
            heap: Vec::with_capacity(capacity),
            index: HashMap::with_capacity(capacity),
            _priorities: PhantomData,
        }
    }

    pub fn new() -> Self {
        BinaryHeapQueue {
            heap: Vec::new(),
            index: HashMap::new(),
            _priorities: PhantomData,
        }
    }

    /// Getting tuple key from node, it is a private helper used when bubblinh up or down 
    fn key(node: &Node<T>) -> (SimTime, u32, u64) {
        (node.time, node.priority, node.event_id)
    }

    /// Helper function to compute parent idx of a node.
    fn parent(idx: HeapIdx) -> HeapIdx {
        if idx == 0 { 0 } else { (idx - 1) / 2 }
    }

    /// Helper function to compute left child idx of a node.
    fn left_child(idx: HeapIdx) -> HeapIdx {
        2 * idx + 1
    }

    /// Helper function to compute right child idx of a node.
    fn right_child(idx: HeapIdx) -> HeapIdx {
        2 * idx + 2
    }

    /// Helper function to see if node has left child
    fn has_left_child(idx: HeapIdx, len: usize) -> bool {
        Self::left_child(idx) < len
    }

    /// Helper function to see if node has right child
    fn has_right_child(idx: HeapIdx, len: usize) -> bool {
        Self::right_child(idx) < len
    }

    /// Helper function that swaps two nodes (given their idxs) while updating hash 
    /// map from event_id to idx
    fn swap(&mut self, a: HeapIdx, b: HeapIdx) {
        self.heap.swap(a, b);
        self.index.insert(self.heap[a].event_id, a);
        self.index.insert(self.heap[b].event_id, b);
    }

    fn bubble_up(&mut self, mut idx: HeapIdx) {
        // If node's key is smaller than parent then heap condition is not met so we
        // swap them (updating the hash map with the new event_id -> heap_idx mapping)
        // and continue doing so until heap condition is respected
        let mut parent_idx = Self::parent(idx);
        while Self::key(&self.heap[idx]) < Self::key(&self.heap[parent_idx]) {
            self.swap(idx, parent_idx);
            idx = parent_idx;
            parent_idx = Self::parent(idx);
        }
    }

    fn bubble_down(&mut self, mut idx: HeapIdx) {
        // If the node has at least one child whose key is smaller than its own the
        // heap condition is not valid, so we swap it with the child with the smallest
        // key and keep swapping until heap condition is met
        let len = self.heap.len();
        loop {
            let mut smallest = idx;

            // If left child exists and its key is smaller than smallest key found so
            // far (i.e. the node's key) then set it as smallest
            if Self::has_left_child(idx, len) {
                let left_idx = Self::left_child(idx);
                if Self::key(&self.heap[left_idx]) < Self::key(&self.heap[smallest]) {
                    smallest = left_idx;
                }
            }

            // If right child exists and its key is smaller than smallest key found so
            // far (i.e. either the node's key or the left child's key) then set it as
            // smallest
            if Self::has_right_child(idx, len) {
                let right_idx = Self::right_child(idx);
                if Self::key(&self.heap[right_idx]) < Self::key(&self.heap[smallest]) {
                    smallest = right_idx;
                }
            }

            // If smallest key out of the three is not the node then heap condition is
            // invalid, so swap it with the node and update the hash map. If smallest
            // key is the node itself then heap is valid and we should stop.
            if smallest == idx {
                break;
            }
            self.swap(idx, smallest);
            idx = smallest;
        }
    }
}

// Implementing EventQueue trait for BinaryHeapQueue
impl<T: Debug + Copy, P: EventPriorities<T>> EventQueue<T, P> for BinaryHeapQueue<T, P> {

    /// New trait just uses BinaryHeapQueue constructor
    fn new() -> Self {
        BinaryHeapQueue::new()
    }

    /// Place node with the event as last child of the heap (and also update event_id
    /// -> heap_idx mapping) and then make it go up until heap condition is met.
    fn insert(&mut self, event_id: u64, time: SimTime, event_type: T) -> Result<(), String> {

        // We check that time is not negative otherwise error
        if time < 0 {
            return Err("Event time cannot be negative".to_string());
        }

        let priority = P::priority(&event_type);
        let idx = self.heap.len();
        self.heap.push(Node { event_id, time, event_type, priority });
        self.index.insert(event_id, idx);
        self.bubble_up(idx);

        Ok(())
    }

    /// Extract the first (minimum key) node. O(log n)
    fn pop_first(&mut self) -> Option<Node<T>> {

        // If heap is empty return None
        if self.heap.is_empty() {
            return None;
        }

        // If heap has just one element remove it and return it
        let node = if self.heap.len() == 1 {
            self.heap.pop().unwrap()
        }
        // Otherwise swap it with last node, pop it, fix up the mapping of the node that
        // is now at the root (since a plain Vec::swap doesn't touch the hash map) and
        // push it down until heap condition is met
        else {
            let last_idx = self.heap.len() - 1;
            self.heap.swap(0, last_idx);
            let node = self.heap.pop().unwrap();
            self.index.insert(self.heap[0].event_id, 0);
            self.bubble_down(0);
            node
        };

        // Remove mapping for the popped event and return it
        self.index.remove(&node.event_id);
        Some(node)
    }

    /// Peek at the time of the first event
    fn peek_first(&self) -> Option<SimTime> {
        self.heap.first().map(|node| node.time)
    }

    fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    fn len(&self) -> usize {
        self.heap.len()
    }

    fn clear(&mut self) {
        self.heap.clear();
        self.index.clear();
    }

    /// Remove a specific event given its event_id. O(log n)
    fn remove(&mut self, _time: SimTime, event_id: u64) -> bool {

        // If entry is not inside the hash map (i.e. the node is not in the heap) exit
        // function otherwise extract its index
        let Some(idx) = self.index.remove(&event_id) else {
            return false;
        };

        // If idx is not valid return (this should not happen)
        if idx >= self.heap.len() {
            return false;
        }

        // If it's the last element, just remove it
        let last_idx = self.heap.len() - 1;
        if idx == last_idx {
            self.heap.pop();
            return true;
        }

        // Otherwise swap with last element (updating only the moved node's mapping)
        // and remove the node that is now at the last element
        self.heap.swap(idx, last_idx);
        let moved_id = self.heap[idx].event_id;
        self.index.insert(moved_id, idx);
        self.heap.pop();

        // The previous last element, which is now at idx, could make the heap
        // condition invalid. Instead of checking whether it violates the condition
        // with its parent or children we simply try to move it up and down.
        self.bubble_up(idx);
        self.bubble_down(idx);
        true
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
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();

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
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();

        queue.insert(next_id(), 20, EventType::Arrival).unwrap();
        queue.insert(next_id(), 10, EventType::Departure).unwrap();

        assert_eq!(queue.peek_first(), Some(10));
        assert_eq!(queue.len(), 2);

        queue.pop_first();
        assert_eq!(queue.peek_first(), Some(20));
    }

    #[test]
    fn test_empty_queue() {
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert!(queue.pop_first().is_none());
    }

    #[test]
    fn test_negative_time_rejected() {
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();
        assert_eq!(
            queue.insert(next_id(), -1, EventType::Arrival),
            Err("Event time cannot be negative".to_string())
        );
    }

    #[test]
    fn test_multiple_events_same_time() {
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();

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
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();

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
    fn test_remove_head() {
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();

        let handle_first = next_id();
        queue.insert(handle_first, 5, EventType::Arrival).unwrap();
        queue.insert(next_id(), 10, EventType::Arrival).unwrap();

        assert_eq!(queue.peek_first(), Some(5));
        assert!(queue.remove(5, handle_first));
        assert_eq!(queue.peek_first(), Some(10));
    }

    #[test]
    fn test_remove_tail() {
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();

        queue.insert(next_id(), 5, EventType::Arrival).unwrap();
        let handle_last = next_id();
        queue.insert(handle_last, 20, EventType::Arrival).unwrap();

        assert_eq!(queue.len(), 2);
        assert!(queue.remove(20, handle_last));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
    }

    #[test]
    fn test_remove_then_pop_rest_stays_consistent() {
        // Regression test: after a pop_first that swaps the root with the last leaf,
        // the moved node's index mapping must be updated before further removals.
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();

        let id1 = next_id();
        let id2 = next_id();

        queue.insert(id1, 5, EventType::Arrival).unwrap();
        queue.insert(id2, 10, EventType::Arrival).unwrap();

        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
        assert!(queue.remove(10, id2));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();

        queue.insert(next_id(), 10, EventType::Arrival).unwrap();
        queue.insert(next_id(), 20, EventType::Arrival).unwrap();

        assert_eq!(queue.len(), 2);
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_insertion_order_preserved() {
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();

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
        let mut queue: BinaryHeapQueue<EventType, TestPriorities> = BinaryHeapQueue::new();

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
}
