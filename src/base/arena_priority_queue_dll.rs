use crate::base::sim_time::SimTime;
use crate::base::priority_queue::Event;

type NodeIdx = usize;
const INVALID_IDX: NodeIdx = usize::MAX;

/// Struct used as an element of the allocator arena, prev and next are simply the vector
/// indices of the previous and following node. Occupied is used to specify whether that
/// slot in the vector holds a node or not. A null pointer is simply an index equal to
/// max of its type (i.e. usize::MAX)
#[derive(Debug)]
struct Node {
    event: Box<dyn Event>,
    prev: NodeIdx,
    next: NodeIdx,
    occupied: bool,
}

// Constructor simply wraps an event trait in a node with no following or previous nodes
impl Node {
    fn new(event: Box<dyn Event>) -> Self {
        Node {
            event,
            prev: INVALID_IDX,
            next: INVALID_IDX,
            occupied: true,
        }
    }
}

/// A handle to a node that can be used to remove it from the queue later.
pub type NodeHandle = NodeIdx;

/// Priority queue implemented as a doubly linked list using an arena allocator.
/// Maintains order by event time: smallest time at the head.
/// Memory is pre-allocated in an arena, avoiding frequent allocations, favoring cache
/// locality and reducing machine-code instructions to iterate over nodes (due to Rc 
/// and Refcell wrappers)
/// It has a vector with Nodes, head and tails are simply indices of head node and tail
/// node, free_list keeps a list of free indices to use to create new nodes
/// Insertion: O(n) worst case, but typically fast
/// Extraction (pop_first): O(1)
/// Removal: O(1) given a NodeHandle
#[derive(Debug)]
pub struct ArenaAcPriorityQueueDLL {
    arena: Vec<Node>,
    head: NodeIdx,
    tail: NodeIdx,
    total_count: usize,
    free_list: Vec<NodeIdx>,
}

impl ArenaAcPriorityQueueDLL {
    /// Create a new arena-based priority queue with an initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {

        // Creating vector with capacity and fill it with unoccupied nodes with a dummy
        // event. Then write all the vector indices in the free list vector.
        let mut arena = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            arena.push(Node {
                event: Box::new(DummyEvent(0)),
                prev: INVALID_IDX,
                next: INVALID_IDX,
                occupied: false,
            });
        }
        let free_list = (0..capacity).collect();

        // Creating the actual struct
        ArenaAcPriorityQueueDLL {
            arena,
            head: INVALID_IDX,
            tail: INVALID_IDX,
            total_count: 0,
            free_list,
        }
    }

    /// Create a new arena-based priority queue with a default capacity of 1024.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Helper function that creates a new Node in the arena and return its index
    fn allocate(&mut self, event: Box<dyn Event>) -> NodeIdx {
        
        // If there is a free node index we pop it from list, create node and return idx
        if let Some(idx) = self.free_list.pop() {
            self.arena[idx] = Node::new(event);
            idx
        } 
        // If no node is available we add an element to the arena, create node there and
        // return that index
        else {
            let idx = self.arena.len();
            self.arena.push(Node::new(event));
            idx
        }
    }

    // Helper function to "destroy" a Node in the arena (marking it as unoccupied)
    fn deallocate(&mut self, idx: NodeIdx) {
        self.arena[idx].occupied = false;
        self.free_list.push(idx);
    }

    /// Helper functions that gets reference to a Node given the index
    fn get_node(&self, idx: NodeIdx) -> &Node {
        &self.arena[idx]
    }

    /// Helper functions that gets mutable reference to a Node given the index
    fn get_node_mut(&mut self, idx: NodeIdx) -> &mut Node {
        &mut self.arena[idx]
    }

    /// Insert an event ordered by time and return a NodeHandle (used to remove Node)
    pub fn insert(&mut self, event: Box<dyn Event>) -> Result<NodeHandle, String> {
        
        // Checks that time is not negative
        let time = event.get_time();
        if time < 0 {
            return Err("Event time cannot be negative".to_string());
        }

        // Creates new node with the event
        let new_idx = self.allocate(event);

        // If there is no head then list is empty so head and tail must point to new node
        if self.head == INVALID_IDX {
            self.head = new_idx;
            self.tail = new_idx;
            self.total_count = 1;
            return Ok(new_idx);
        }

        // Otherwise iteratore over nodes until you find one that has time >= new time
        let mut current = self.head;
        let mut inserted = false;
        while current != INVALID_IDX {

            // Checks if current node has time >= new node time, if so get next node and
            // prev node, link next node to current and if prev node exists do the same
            // otherwise set current as head
            let curr_time = self.get_node(current).event.get_time();
            if curr_time >= time {
                let prev_idx = self.get_node(current).prev;

                self.get_node_mut(current).prev = new_idx;
                self.get_node_mut(new_idx).next = current;

                if prev_idx != INVALID_IDX {
                    self.get_node_mut(new_idx).prev = prev_idx;
                    self.get_node_mut(prev_idx).next = new_idx;
                } else {
                    self.head = new_idx;
                    self.get_node_mut(new_idx).prev = INVALID_IDX;
                }

                inserted = true;
                break;
            }

            // Go to next node
            current = self.get_node(current).next;
        }

        // If at the end of the scan new node was not inserted it means it has to be 
        // inserted at the end, so link it with tail and set it as new tail
        if !inserted {
            if self.tail != INVALID_IDX {
                self.get_node_mut(self.tail).next = new_idx;
                self.get_node_mut(new_idx).prev = self.tail;
            }
            self.tail = new_idx;
        }

        self.total_count += 1;
        Ok(new_idx)
    }

    /// Extract the first (minimum time) event
    pub fn pop_first(&mut self) -> Option<Box<dyn Event>> {

        // If empty list return None
        if self.head == INVALID_IDX {
            return None;
        }

        // Get id of current head and of its next node
        let extracted_idx = self.head;
        let new_head_idx = self.get_node(extracted_idx).next;

        // If next node index is invalid then list is empty, head and tail have to be
        // set to null. Otherwise head will point to next node index and its prev cleared
        if new_head_idx != INVALID_IDX {
            self.head = new_head_idx;
            self.get_node_mut(new_head_idx).prev = INVALID_IDX;
        } else {
            self.tail = INVALID_IDX;
            self.head = INVALID_IDX;
        }

        self.total_count -= 1;
        let event = self.get_node(extracted_idx).event.clone_box();
        self.deallocate(extracted_idx);
        Some(event)
    }

    /// Peek at the time of the first event.
    pub fn peek_first(&self) -> Option<SimTime> {
        if self.head == INVALID_IDX {
            None
        } else {
            Some(self.get_node(self.head).event.get_time())
        }
    }

    /// Debug: return all events in queue order
    #[cfg(test)]
    fn debug_all_times(&self) -> Vec<SimTime> {
        let mut times = Vec::new();
        let mut current = self.head;
        while current != INVALID_IDX {
            times.push(self.get_node(current).event.get_time());
            current = self.get_node(current).next;
        }
        times
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.head == INVALID_IDX
    }

    /// Number of events in queue
    pub fn len(&self) -> usize {
        self.total_count
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.head = INVALID_IDX;
        self.tail = INVALID_IDX;
        self.total_count = 0;
        self.free_list.clear();
        for (i, node) in self.arena.iter_mut().enumerate() {
            node.occupied = false;
            self.free_list.push(i);
        }
    }

    /// Remove a specific event given its NodeHandle. O(1)
    pub fn remove(&mut self, node_idx: NodeHandle) {
        let prev_idx = self.get_node(node_idx).prev;
        let next_idx = self.get_node(node_idx).next;

        if next_idx != INVALID_IDX {
            self.get_node_mut(next_idx).prev = prev_idx;
        } else {
            self.tail = prev_idx;
        }

        if prev_idx != INVALID_IDX {
            self.get_node_mut(prev_idx).next = next_idx;
        } else {
            self.head = next_idx;
        }

        self.total_count -= 1;
        self.deallocate(node_idx);
    }
}

// Dummy event for arena initialization
#[derive(Debug, Clone)]
struct DummyEvent(SimTime);

impl Event for DummyEvent {
    fn get_time(&self) -> SimTime {
        self.0
    }

    fn set_time(&mut self, time: SimTime) {
        self.0 = time;
    }

    fn get_id(&self) -> u64 {
        0
    }

    fn set_id(&mut self, _id: u64) {}

    fn doit(&mut self) -> (bool, Vec<Box<dyn Event>>) {
        (true, vec![])
    }

    fn clone_box(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::event::BasicEvent;

    #[test]
    fn test_insert_and_pop() {
        let mut queue = ArenaAcPriorityQueueDLL::new();

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
        let mut queue = ArenaAcPriorityQueueDLL::new();

        queue.insert(Box::new(BasicEvent::new(20))).unwrap();
        queue.insert(Box::new(BasicEvent::new(10))).unwrap();

        assert_eq!(queue.peek_first(), Some(10));
        assert_eq!(queue.len(), 2);

        queue.pop_first();
        assert_eq!(queue.peek_first(), Some(20));
    }

    #[test]
    fn test_empty_queue() {
        let mut queue = ArenaAcPriorityQueueDLL::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.pop_first().map(|e| e.get_time()), None);
    }

    #[test]
    fn test_negative_time_rejected() {
        let mut queue = ArenaAcPriorityQueueDLL::new();
        let event = BasicEvent::new(-1);
        let result = queue.insert(Box::new(event));
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_events_same_time() {
        let mut queue = ArenaAcPriorityQueueDLL::new();

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
        let mut queue = ArenaAcPriorityQueueDLL::new();

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
        let mut queue = ArenaAcPriorityQueueDLL::new();

        let handle_first = queue.insert(Box::new(BasicEvent::new(5))).unwrap();
        let _handle_second = queue.insert(Box::new(BasicEvent::new(10))).unwrap();

        assert_eq!(queue.peek_first(), Some(5));
        queue.remove(handle_first);
        assert_eq!(queue.peek_first(), Some(10));
    }

    #[test]
    fn test_remove_tail() {
        let mut queue = ArenaAcPriorityQueueDLL::new();

        let _handle_first = queue.insert(Box::new(BasicEvent::new(5))).unwrap();
        let handle_last = queue.insert(Box::new(BasicEvent::new(20))).unwrap();

        assert_eq!(queue.len(), 2);
        queue.remove(handle_last);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(5));
    }

    #[test]
    fn test_clear() {
        let mut queue = ArenaAcPriorityQueueDLL::new();

        queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        queue.insert(Box::new(BasicEvent::new(20))).unwrap();

        assert_eq!(queue.len(), 2);
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_insertion_order_preserved() {
        let mut queue = ArenaAcPriorityQueueDLL::new();

        let _h1 = queue.insert(Box::new(BasicEvent::new(30))).unwrap();
        let _h2 = queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        let _h3 = queue.insert(Box::new(BasicEvent::new(20))).unwrap();
        let _h4 = queue.insert(Box::new(BasicEvent::new(15))).unwrap();

        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(15));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(20));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(30));
    }

    #[test]
    fn test_arena_reuse() {
        let mut queue = ArenaAcPriorityQueueDLL::with_capacity(10);

        let h1 = queue.insert(Box::new(BasicEvent::new(5))).unwrap();
        let h2 = queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        let h3 = queue.insert(Box::new(BasicEvent::new(15))).unwrap();

        queue.remove(h2);
        queue.remove(h1);

        let _h4 = queue.insert(Box::new(BasicEvent::new(7))).unwrap();
        queue.remove(h3);
        let _h5 = queue.insert(Box::new(BasicEvent::new(12))).unwrap();

        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(7));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(12));
        assert!(queue.pop_first().is_none());
    }

    #[test]
    fn test_extract_order_sporadic() {
        use crate::base::sporadic_event::SporadicEvent;
        use crate::base::random_variable::Uniform;

        let mut queue = ArenaAcPriorityQueueDLL::with_capacity(100);

        // Insert 10 events with the same initial time
        for i in 0..10 {
            let event = SporadicEvent::new(
                0,
                Box::new(Uniform::new(1, 1000, i as u64)),
            );
            queue.insert(Box::new(event)).unwrap();
        }

        // Pop all events and verify they come out in order
        let mut prev_time = i64::MIN;
        for _ in 0..10 {
            let event = queue.pop_first().expect("should have events");
            let time = event.get_time();
            assert!(time >= prev_time, "Event time {} should be >= previous {}", time, prev_time);
            prev_time = time;
        }
    }

    #[test]
    fn test_multiple_reschedules_order() {
        use crate::base::sporadic_event::SporadicEvent;
        use crate::base::random_variable::Deterministic;

        let mut queue = ArenaAcPriorityQueueDLL::with_capacity(100);

        // Create events that will reschedule predictably, all starting at time 100
        queue.insert(Box::new(SporadicEvent::new(
            100,
            Box::new(Deterministic::new(10)),
        ))).unwrap();
        queue.insert(Box::new(SporadicEvent::new(
            100,
            Box::new(Deterministic::new(5)),
        ))).unwrap();
        queue.insert(Box::new(SporadicEvent::new(
            100,
            Box::new(Deterministic::new(15)),
        ))).unwrap();

        // Pop all 3 events with time=100, reschedule each
        for _ in 0..3 {
            let mut e = queue.pop_first().unwrap();
            assert_eq!(e.get_time(), 100, "Should extract time=100 event");
            e.doit(); // time becomes 100 + delta
            queue.insert(e).unwrap();
        }

        // Now queue should have events with times 105, 110, 115
        // Verify they extract in sorted order
        let e1 = queue.pop_first().unwrap();
        assert_eq!(e1.get_time(), 105, "Should extract time=105 event, got {}", e1.get_time());
        let e2 = queue.pop_first().unwrap();
        assert_eq!(e2.get_time(), 110, "Should extract time=110 event, got {}", e2.get_time());
        let e3 = queue.pop_first().unwrap();
        assert_eq!(e3.get_time(), 115, "Should extract time=115 event, got {}", e3.get_time());
    }

    #[test]
    fn test_benchmark_extract_order() {
        use crate::base::sporadic_event::SporadicEvent;
        use crate::base::random_variable::Uniform;

        let mut queue = ArenaAcPriorityQueueDLL::with_capacity(1000);

        // Simulate the benchmark's population phase: insert, pop, doit, reinsert
        for i in 0..100 {
            let event = SporadicEvent::new(
                0,
                Box::new(Uniform::new(1, 1000, i as u64)),
            );
            queue.insert(Box::new(event)).unwrap();
            let mut event = queue.pop_first().unwrap();
            let (_, _) = event.doit();
            queue.insert(event).unwrap();
        }

        // Now run multiple cycles like the benchmark does
        for cycle in 0..5 {
            let mut times = Vec::new();

            // Pop all events and record times
            while let Some(event) = queue.pop_first() {
                times.push(event.get_time());
            }

            // Verify all times are in sorted order
            for i in 1..times.len() {
                assert!(times[i] >= times[i-1],
                    "Cycle {}: Events not in order! {} > {}",
                    cycle, times[i], times[i-1]);
            }

            // Re-insert all events after doit
            for _ in 0..times.len() {
                let mut event = SporadicEvent::new(0, Box::new(Uniform::new(1, 1000, 42)));
                // Just use consistent times for this test
                event.set_time(times[0]);  // Will be overwritten anyway
                queue.insert(Box::new(event)).unwrap();
            }
        }
    }
}
