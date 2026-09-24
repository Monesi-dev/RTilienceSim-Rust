
use std::fmt::Debug;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
pub type SimTime = i64;

pub trait Event: Debug {
    fn get_time(&self) -> SimTime;
    fn set_time(&mut self, time: SimTime);
    fn get_id(&self) -> u64;
    fn set_id(&mut self, id: u64);
    fn doit(&mut self) -> (bool, Vec<Box<dyn Event>>);
    fn clone_box(&self) -> Box<dyn Event>;
}

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