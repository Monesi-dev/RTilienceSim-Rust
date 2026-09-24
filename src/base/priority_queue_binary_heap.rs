use crate::base::sim_time::SimTime;
use crate::base::priority_queue::Event;
use std::collections::HashMap;

pub type NodeHandle = u64;

/// Priority queue implemented as an indexed binary heap.
/// You keep values in a binary tree where each node is smaller than its two children,
/// smallest node will be at the top. The tree is implemented as vector (node ad index
/// idx has as children the nodes at indices 2*idx+1 and 2*idx+2)
/// A hash map is used to map in O(1) the event id with the index idx of its node in 
/// the heap so removal can be made in O(logN) instead of O(N)
/// Insertion: O(logN)
/// Extraction (pop_first): O(logN)
/// Removal: O(logN) given a NodeHandle
pub struct PriorityQueueBinaryHeap {
    heap: Vec<Box<dyn Event>>,
    id_to_index: HashMap<u64, usize>,
}

impl PriorityQueueBinaryHeap {
    pub fn new() -> Self {
        PriorityQueueBinaryHeap {
            heap: Vec::new(),
            id_to_index: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        PriorityQueueBinaryHeap {
            heap: Vec::with_capacity(capacity),
            id_to_index: HashMap::with_capacity(capacity),
        }
    }

    fn parent(idx: usize) -> usize {
        if idx == 0 { 0 } else { (idx - 1) / 2 }
    }

    fn left_child(idx: usize) -> usize {
        2 * idx + 1
    }

    fn right_child(idx: usize) -> usize {
        2 * idx + 2
    }

    fn has_left_child(idx: usize, len: usize) -> bool {
        Self::left_child(idx) < len
    }

    fn has_right_child(idx: usize, len: usize) -> bool {
        Self::right_child(idx) < len
    }

    fn bubble_up(&mut self, mut idx: usize) {
        // If node's value is smaller than parent then heap condition is not met so we
        // swap them (update hash map with new event_id -> heap_id mapping) and continue 
        // doing so until heap condition is respected
        let parent_idx = Self::parent(idx);
        while self.heap[idx].get_time() < self.heap[parent_idx].get_time() {
            self.heap.swap(idx, parent_idx);
            self.id_to_index.insert(self.heap[idx].get_id(), idx);
            self.id_to_index.insert(self.heap[parent_idx].get_id(), parent_idx);
            idx = parent_idx;
            let parent_idx = Self::parent(idx);
        }
    }

    fn bubble_down(&mut self, mut idx: usize) {
        // If the node has at least one child whose value is smaller than himself the
        // heap condition is not valid, so we swap it with the child with smallest value
        // and keep swapping until heap condition is met
        let len = self.heap.len();
        let mut valid_heap = false;
        while valid_heap == false {
            let mut smallest = idx;

            // If left child exists and is smaller than smallest value found so far (i.e.
            // the node's value) then set it as smallest value
            if Self::has_left_child(idx, len) {
                let left_idx = Self::left_child(idx);
                if self.heap[left_idx].get_time() < self.heap[smallest].get_time() {
                    smallest = left_idx;
                }
            }

            // If right child exists and is smaller than smallest value found so far (i.e.
            // either the node's value or the left child's value) then set is as smallest
            if Self::has_right_child(idx, len) {
                let right_idx = Self::right_child(idx);
                if self.heap[right_idx].get_time() < self.heap[smallest].get_time() {
                    smallest = right_idx;
                }
            }

            // If smallest value out of the three is not the node then heap condition is
            // invalid, so swap it with node and update hash map. If smallest value is the
            // node itself then heap is valid and we shoul stop.
            if smallest != idx {
                self.heap.swap(idx, smallest);
                self.id_to_index.insert(self.heap[idx].get_id(), idx);
                self.id_to_index.insert(self.heap[smallest].get_id(), smallest);
                idx = smallest;
            } else {
                valid_heap = true;
            }
        }
    }

    /// Insert an event and return a handle for later removal
    pub fn insert(&mut self, event: Box<dyn Event>) -> Result<NodeHandle, String> {
        
        // We check that time is not negative otherwise error
        let time = event.get_time();
        if time < 0 {
            return Err("Event time cannot be negative".to_string());
        }

        // Place node with the event as last child of the heap (and also update event id
        // -> heap idx mapping) and then make it go up until heap condition is met (call
        // the bubble_up method)
        let event_id = event.get_id();
        let idx = self.heap.len();
        self.heap.push(event);
        self.id_to_index.insert(event_id, idx);
        self.bubble_up(idx);

        Ok(event_id)
    }

    /// Extract the first (minimum time) event. O(log n)
    pub fn pop_first(&mut self) -> Option<Box<dyn Event>> {

        // If tree is empty return None
        if self.heap.is_empty() {
            return None;
        }

        // If heap has just one element remove it and return it
        let event = if self.heap.len() == 1 {
            self.heap.pop().unwrap()
        } 
        // Otherwise swap it with last node, pop it and push down previous last node 
        // (which is now on top) until heap condition is met
        else {
            let last_idx = self.heap.len() - 1;
            self.heap.swap(0, last_idx);
            let event = self.heap.pop().unwrap();
            self.bubble_down(0);
            event
        };

        // Remove mapping of for removed event and return event
        self.id_to_index.remove(&event.get_id());
        Some(event)
    }

    /// Peek at the time of the first event
    pub fn peek_first(&self) -> Option<SimTime> {
        self.heap.first().map(|event| event.get_time())
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Number of events in queue
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.heap.clear();
        self.id_to_index.clear();
    }

    /// Remove a specific event given its NodeHandle. O(log n)
    pub fn remove(&mut self, handle: NodeHandle) -> () {

        // If entry is not inside the hash map (i.e. the node is not in the heap) exit
        // function otherwise extract its id
        let Some(idx) = self.id_to_index.get(&handle).copied() else {
            return;
        };
        
        // If idx is not valid return (this should not happen)
        if idx >= self.heap.len() {
            return
        }

        // Remove the index from the map
        self.id_to_index.remove(&handle);

        // If it's the last element, just remove it
        let last_idx = self.heap.len() - 1;
        if idx == last_idx {
            self.heap.pop();
            return;
        }

        // Otherwise swap with last element (updating its mapping) and remove node that 
        // is now at last element
        self.heap.swap(idx, last_idx);
        let swapped_id = self.heap[idx].get_id();
        self.id_to_index.insert(swapped_id, idx);
        self.heap.pop();

        // Previous last element, which is now at idx could make heap condition invalid,
        // instead of checking whether it violates condition with its parent or children
        // we simply try to move it up and move it down, only when we take the right 
        // direction (up or down) the node will be swapped
        self.bubble_up(idx);
        self.bubble_down(idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::event::BasicEvent;

    #[test]
    fn test_insert_and_pop() {
        let mut queue = PriorityQueueBinaryHeap::new();

        queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        queue.insert(Box::new(BasicEvent::new(5))).unwrap();
        queue.insert(Box::new(BasicEvent::new(15))).unwrap();

        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(5));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(15));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), None);
    }

    #[test]
    fn test_peek_first() {
        let mut queue = PriorityQueueBinaryHeap::new();

        queue.insert(Box::new(BasicEvent::new(20))).unwrap();
        queue.insert(Box::new(BasicEvent::new(10))).unwrap();

        assert_eq!(queue.peek_first(), Some(10));
        assert_eq!(queue.len(), 2);

        queue.pop_first();
        assert_eq!(queue.peek_first(), Some(20));
    }

    #[test]
    fn test_empty_queue() {
        let mut queue: PriorityQueueBinaryHeap = PriorityQueueBinaryHeap::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.pop_first().map(|e| e.get_time()), None);
    }

    #[test]
    fn test_negative_time_rejected() {
        let mut queue = PriorityQueueBinaryHeap::new();
        let event = BasicEvent::new(-1);
        let result = queue.insert(Box::new(event));
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_events_same_time() {
        let mut queue = PriorityQueueBinaryHeap::new();

        queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        queue.insert(Box::new(BasicEvent::new(5))).unwrap();

        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(5));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert!(queue.pop_first().is_none());
    }

    #[test]
    fn test_remove_event() {
        let mut queue = PriorityQueueBinaryHeap::new();

        let handle1 = queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        let handle2 = queue.insert(Box::new(BasicEvent::new(20))).unwrap();
        let handle3 = queue.insert(Box::new(BasicEvent::new(30))).unwrap();
        let _handle4 = queue.insert(Box::new(BasicEvent::new(25))).unwrap();
        let _handle5 = queue.insert(Box::new(BasicEvent::new(15))).unwrap();

        assert_eq!(queue.len(), 5);
        queue.remove(handle2);
        assert_eq!(queue.len(), 4);
        queue.remove(handle1);
        assert_eq!(queue.len(), 3);
        queue.remove(handle3);
        assert_eq!(queue.len(), 2);

        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(15));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(25));
        assert!(queue.pop_first().is_none());
    }

    #[test]
    fn test_remove_head() {
        let mut queue = PriorityQueueBinaryHeap::new();

        let handle_first = queue.insert(Box::new(BasicEvent::new(5))).unwrap();
        let _handle_second = queue.insert(Box::new(BasicEvent::new(10))).unwrap();

        assert_eq!(queue.peek_first(), Some(5));
        queue.remove(handle_first);
        assert_eq!(queue.peek_first(), Some(10));
    }

    #[test]
    fn test_remove_tail() {
        let mut queue = PriorityQueueBinaryHeap::new();

        let _handle_first = queue.insert(Box::new(BasicEvent::new(5))).unwrap();
        let handle_last = queue.insert(Box::new(BasicEvent::new(20))).unwrap();

        assert_eq!(queue.len(), 2);
        queue.remove(handle_last);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(5));
    }

    #[test]
    fn test_clear() {
        let mut queue = PriorityQueueBinaryHeap::new();

        queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        queue.insert(Box::new(BasicEvent::new(20))).unwrap();

        assert_eq!(queue.len(), 2);
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_insertion_order_preserved() {
        let mut queue = PriorityQueueBinaryHeap::new();

        let _h1 = queue.insert(Box::new(BasicEvent::new(30))).unwrap();
        let _h2 = queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        let _h3 = queue.insert(Box::new(BasicEvent::new(20))).unwrap();
        let _h4 = queue.insert(Box::new(BasicEvent::new(15))).unwrap();

        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(15));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(20));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(30));
    }
}
