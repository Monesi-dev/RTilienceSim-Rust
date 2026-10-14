//! Arena-based priority queue with HashMap indexing
//! O(1) pop_first, O(1) removal, O(n) insertion

use crate::base::sim_time::SimTime;
use super::priority_queue::Node;
use super::priority_queue_trait::{EventQueue, EventPriorities};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

type NodeIdx = usize;
const INVALID_IDX: NodeIdx = usize::MAX;

/// Arena-based DLL Node struct
/// It holds the <event_id, time, event_type> triplet that an abstract node of a priority
/// queue should contain
/// `prev`, `next` are used as links of the DLL (they are idx of vector element with node)
#[derive(Debug, Clone)]
struct ArenaNode<T: Debug + Copy> {
    event_id: u64,
    time: SimTime,
    event_type: T,
    /// Cached result of `P::priority(&event_type)` at insertion time, used to tie-break
    /// nodes sharing the same `time` (lower value sorts earlier in the DLL).
    priority: u32,
    prev: NodeIdx,
    next: NodeIdx,
}

/// Arena-based priority queue with HashMap indexing for O(1) removal
/// > Insertion: O(n) potentially scan all list to find where to put node
/// > Pop first: O(1) constant time
/// > Removal: O(1) HashMap lookup to get node and then unlink
pub struct ArenaQueue<T: Debug + Copy, P: EventPriorities<T>> {
    arena: Vec<Option<ArenaNode<T>>>, // Pre-allocated vector to store nodes
    index: HashMap<u64, NodeIdx>,     // HashMap event_id->node_idx to have O(1) removal
    head: NodeIdx,                    // Idx of head node
    tail: NodeIdx,                    // Idx of tail node
    total_count: usize,               // Size of DLL
    free_list: Vec<NodeIdx>,          // Vector with Idx that are free
    _priorities: PhantomData<P>,
}

// Implementing methods for ArenaQueue that will be used as private helpers when
// implementing the EventQueue interface
impl<T: Debug + Copy, P: EventPriorities<T>> ArenaQueue<T, P> {

    /// Given a size you return an empty ArenaDLL with a size pre-allocated vector
    pub fn with_capacity(capacity: usize) -> Self {

        // Pre-allocate vector filling with None and put all indices in free list
        let arena = vec![None; capacity];
        let free_list = (0..capacity).collect();

        // Create ArenaQueue struct
        ArenaQueue {
            arena,
            index: HashMap::new(),
            head: INVALID_IDX,
            tail: INVALID_IDX,
            total_count: 0,
            free_list,
            _priorities: PhantomData,
        }
    }

    /// Initialize an ArenaDLL with 1024 pre-allocated vector
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Given <event_id, time, event_type, priority> it finds an empty cell of the
    /// ArenaAllocator creates a node in it and returns the index. The node is just
    /// allocated but not added to the DLL
    fn allocate(&mut self, event_id: u64, time: SimTime, event_type: T, priority: u32) -> NodeIdx {

        // If there is space in the vector pop a free index, allocate node there
        if let Some(idx) = self.free_list.pop() {
            self.arena[idx] = Some(ArenaNode {
                event_id,
                time,
                event_type,
                priority,
                prev: INVALID_IDX,
                next: INVALID_IDX,
            });
            idx
        }
        // Otherwise add a new element to the array and allocate there
        else {
            let idx = self.arena.len();
            self.arena.push(Some(ArenaNode {
                event_id,
                time,
                event_type,
                priority,
                prev: INVALID_IDX,
                next: INVALID_IDX,
            }));
            idx
        }
    }

    /// Given the idx of an ArenaAllocator cells it de-allocates the Node (i.e. sets the
    /// Option to None and add idx in the free list)
    fn deallocate(&mut self, idx: NodeIdx) {
        if idx < self.arena.len() {
            self.arena[idx] = None;
            self.free_list.push(idx);
        }
    }
}

// Implementing EventQueue trait for ArenaQueue
impl<T: Debug + Copy, P: EventPriorities<T>> EventQueue<T, P> for ArenaQueue<T, P> {

    /// New trait just uses ArenaQueue constructor
    fn new() -> Self {
        ArenaQueue::new()
    }

    /// Given the <event_id, time, event_type> triplet it creates a Node to be added in
    /// the DLL.
    fn insert(&mut self, event_id: u64, time: SimTime, event_type: T) -> Result<(), String> {

        // Discards negative time
        if time < 0 {
            return Err("Event time cannot be negative".to_string());
        }

        // Allocates new node and create event_id->node_idx mapping
        let priority = P::priority(&event_type);
        let new_idx = self.allocate(event_id, time, event_type, priority);
        self.index.insert(event_id, new_idx);

        // If list is empty just put it as head
        if self.head == INVALID_IDX {
            self.head = new_idx;
            self.tail = new_idx;
            self.total_count += 1;
            return Ok(());
        }

        // Otherwise iterate from head onwards until you find a node with (time, priority,
        // event_id) >= than the new node's key and insert right before it. Comparing the
        // full tuple (rather than just time) ties-breaks equal time events using priority,
        // then using event_id if priority is also equal.
        let mut current = self.head;
        loop {

            // Get reference to node, copy time and if key >= new node key then add
            // new node right before it
            let current_node = self.arena[current].as_ref().unwrap();
            let current_key = (current_node.time, current_node.priority, current_node.event_id);
            if (time, priority, event_id) <= current_key {

                // Save previous node index of current node. Then link current node with 
                // new node 
                let prev_idx = self.arena[current].as_ref().unwrap().prev;
                self.arena[current].as_mut().unwrap().prev = new_idx;
                self.arena[new_idx].as_mut().unwrap().next = current;

                // Link new node with previous node. If previous node doesn't exist
                // then set new node as head.
                self.arena[new_idx].as_mut().unwrap().prev = prev_idx;
                if prev_idx != INVALID_IDX {
                    self.arena[prev_idx].as_mut().unwrap().next = new_idx;
                } else {
                    self.head = new_idx;
                }
                break;
            }

            // If time is not >= new node time then get idx of following node. If index
            // is invalid new node has to be inserted in tail so link new node with 
            // current node and set tail index.
            let next_idx = self.arena[current].as_ref().unwrap().next;
            if next_idx == INVALID_IDX {
                self.arena[current].as_mut().unwrap().next = new_idx;
                self.arena[new_idx].as_mut().unwrap().prev = current;
                self.tail = new_idx;
                break;
            }
            current = next_idx;
        }

        self.total_count += 1;
        Ok(())
    }

    /// Extracts node with smallest time (i.e. head of DLL)
    fn pop_first(&mut self) -> Option<Node<T>> {

        // If DLL is empty return none
        if self.head == INVALID_IDX {
            return None;
        }

        // Get reference to head node and extract <event_id, time, event_type, priority>
        let idx = self.head;
        let node = self.arena[idx].as_ref().unwrap();
        let event_id = node.event_id;
        let time = node.time;
        let event_type = node.event_type;
        let priority = node.priority;

        // Remove head node if following node exists then sets it as head (and remove its
        // prev idx pointer). Otherwise just make head and tail invalid as list is empty
        let next_idx = node.next;
        if next_idx != INVALID_IDX {
            self.arena[next_idx].as_mut().unwrap().prev = INVALID_IDX;
            self.head = next_idx;
        } else {
            self.head = INVALID_IDX;
            self.tail = INVALID_IDX;
        }

        // Remove event_id->node_idx mapping and deallocate node
        self.index.remove(&event_id);
        self.deallocate(idx);
        self.total_count -= 1;

        // Return <event_id, time, event_type, priority> wrapped in Node struct
        Some(Node {
            event_id,
            time,
            event_type,
            priority,
        })
    }

    /// Gets smallest time without removing Node (i.e. reads time of head)
    fn peek_first(&self) -> Option<SimTime> {
        if self.head == INVALID_IDX {
            None
        } else {
            self.arena[self.head].as_ref().map(|n| n.time)
        }
    }

    fn is_empty(&self) -> bool {
        self.head == INVALID_IDX
    }

    fn len(&self) -> usize {
        self.total_count
    }

    fn clear(&mut self) {
        self.arena.clear();
        self.index.clear();
        self.head = INVALID_IDX;
        self.tail = INVALID_IDX;
        self.total_count = 0;
        self.free_list.clear();
    }

    /// Given <time, event_id> it removes the node, DLL only needs event_id but some 
    /// other trait implementations also need time.
    /// You use hashmap event_id->node_idx to access node and then remove it.
    fn remove(&mut self, _time: SimTime, event_id: u64) -> bool {
        
        // Try to access node_idx with hash map if there is no mapping return false
        let Some(idx) = self.index.remove(&event_id) else {
            return false;
        };

        // If index is not valid return false
        if idx >= self.arena.len() || self.arena[idx].is_none() {
            return false;
        }

        // Gets indices of following and previous node
        let prev_idx = self.arena[idx].as_ref().unwrap().prev;
        let next_idx = self.arena[idx].as_ref().unwrap().next;

        // If previous node exists then sets its next pointer to following node. If it
        // doesn't exists then you are removing head, so head should point to following node
        if prev_idx != INVALID_IDX {
            self.arena[prev_idx].as_mut().unwrap().next = next_idx;
        } else {
            self.head = next_idx;
        }

        // If following node exists then sets its prev pointer to previous node. If it 
        // doesn't exist then you are removing tail, so set tail to previous node
        if next_idx != INVALID_IDX {
            self.arena[next_idx].as_mut().unwrap().prev = prev_idx;
        } else {
            self.tail = prev_idx;
        }

        // De-allocated removed node and decrement node count
        self.deallocate(idx);
        self.total_count -= 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    enum TestEvent {
        Type1,
        Type2,
    }

    #[derive(Debug, Clone, Copy)]
    struct TestPriorities;

    impl EventPriorities<TestEvent> for TestPriorities {
        fn priority(event_type: &TestEvent) -> u32 {
            match event_type {
                TestEvent::Type1 => 0,
                TestEvent::Type2 => 1,
            }
        }
    }

    #[test]
    fn test_insert_and_pop() {
        let mut queue: ArenaQueue<TestEvent, TestPriorities> = ArenaQueue::new();
        queue.insert(1, 10, TestEvent::Type1).unwrap();
        queue.insert(2, 5, TestEvent::Type2).unwrap();
        queue.insert(3, 15, TestEvent::Type1).unwrap();

        assert_eq!(queue.pop_first().map(|n| n.time), Some(5));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(15));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_remove() {
        let mut queue: ArenaQueue<TestEvent, TestPriorities> = ArenaQueue::new();
        queue.insert(1, 10, TestEvent::Type1).unwrap();
        queue.insert(2, 20, TestEvent::Type2).unwrap();
        queue.insert(3, 30, TestEvent::Type1).unwrap();

        assert_eq!(queue.len(), 3);
        assert!(queue.remove(20, 2));
        assert_eq!(queue.len(), 2);

        assert_eq!(queue.pop_first().map(|n| n.time), Some(10));
        assert_eq!(queue.pop_first().map(|n| n.time), Some(30));
    }
}
