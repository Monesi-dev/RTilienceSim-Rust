use crate::base::sim_time::SimTime;
use crate::base::priority_queue::Event;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type NodePtr = Rc<RefCell<Node>>;
type WeakNodePtr = Weak<RefCell<Node>>;

/// A handle to a node that can be used to remove it from the queue later.
pub type NodeHandle = NodePtr;

#[derive(Debug)]
pub struct Node {
    event: Box<dyn Event>,
    prev: Option<WeakNodePtr>,  // Weak reference to prevent cycles
    next: Option<NodePtr>,              // Strong reference
}

impl Node {
    fn new(event: Box<dyn Event>) -> Self {
        Node {
            event,
            prev: None,
            next: None,
        }
    }
}

/// Priority queue implemented as a doubly linked list.
/// Maintains order by event time: smallest time at the head.
/// Insertion: O(n) worst case, but typically fast
/// Extraction (pop_first): O(1)
/// Removal: O(1) given a NodeHandle
#[derive(Debug)]
pub struct PriorityQueueDLL {
    head: Option<NodePtr>,
    tail: Option<NodePtr>,
    total_count: usize,
}

impl PriorityQueueDLL {
    pub fn new() -> Self {
        PriorityQueueDLL {
            head: None,
            tail: None,
            total_count: 0,
        }
    }

    /// Insert an event in sorted order by time and returns another owner of the node 
    /// as it can be used to delete the node. 
    pub fn insert(&mut self, event: Box<dyn Event>) -> Result<NodeHandle, String> {

        // If time is negative return error string
        let time = event.get_time();
        if time < 0 {
            return Err("Event time cannot be negative".to_string());
        }

        // Create new node
        let new_node = Rc::new(RefCell::new(Node::new(event)));

        // If list is empty, insert as first node
        if self.head.is_none() {
            self.head = Some(Rc::clone(&new_node));
            self.tail = Some(Rc::clone(&new_node));
            self.total_count = 1;
            return Ok(new_node);
        }

        // If list is not empty we scan it looking for first node with time >= curr_time
        // and when we find it we insert new node right before it. If node is not found
        // then new node should be added at the end of the list
        let mut current = self.head.clone();
        let mut inserted = false;

        while let Some(curr_node) = current {

            // Checks if node is the first to have a >= time
            let curr_time = curr_node.borrow().event.get_time();
            if curr_time >= time {

                // Cloning the Option around prev node so I add a new owner to wrapped
                // Rc and don't have to bother with stealing ownership, new owner will
                // be destroyed at the end of this block
                let prev_node_wrapper = curr_node.borrow_mut().prev.clone();

                // Making curr_node previous pointer point to new node and making new 
                // node next pointer point to curr_node, i.e. connecting next node with
                // new node
                curr_node.borrow_mut().prev = Some(Rc::downgrade(&new_node));
                new_node.borrow_mut().next = Some(Rc::clone(&curr_node));

                // If there is a previous node to connect current with we make new node
                // previous pointer point to that node and we make that node next pointer
                // point to new node, i.e. we connect new node to prev node
                if let Some(prev_node) = prev_node_wrapper {
                    let prev_node = prev_node.upgrade().unwrap();
                    new_node.borrow_mut().prev = Some(Rc::downgrade(&prev_node));
                    prev_node.borrow_mut().next = Some(Rc::clone(&new_node));
                }

                // If there is not previous node then new node will become the new head
                else {
                    self.head = Some(Rc::clone(&new_node));
                    new_node.borrow_mut().prev = None;
                }

                inserted = true;
                break;
            }

            // We clone the Option around nnext node (making a new owner of the Rc) so 
            // we don't have to worry about stealing ownership and this owner will be
            // destroyed after scanning the node
            current = curr_node.borrow_mut().next.clone();
        }

        // If no node has been found then add node as new tail, so make tail next pointer
        // point to new node and new node prev pointer point to old tail
        if !inserted {

            let tail_wrapper = self.tail.clone();
            if let Some(tail_node) = tail_wrapper {
                tail_node.borrow_mut().next = Some(Rc::clone(&new_node));
                new_node.borrow_mut().prev = Some(Rc::downgrade(&tail_node));
            }
            self.tail = Some(Rc::clone(&new_node));
        }

        self.total_count += 1;
        Ok(new_node)
    }

    /// Extract the first (minimum time) event. O(1)
    pub fn pop_first(&mut self) -> Option<Box<dyn Event>> {

        // I clone the Option<> around the Head of list and try to unwrap the value (If
        // there is no value inside then list is empty so it is correct to retunr None)
        let extracted_head_node = self.head.clone()?;

        // I clone the Option<> around the node following the previous Head, if next 
        // node exists then head of list should point to that node otherwise list becomes
        // empty
        let new_head_wrapped = extracted_head_node.borrow_mut().next.clone();
        if let Some(new_head) = new_head_wrapped {
            new_head.borrow_mut().prev = None;
            self.head = Some(new_head);
        }
        else {
            self.tail = None;
            self.head = None;
        }

        // Decrement list counter and clone the event inside the extracted node, I cannot
        // pass Box<dyn Event> ownership as borrow_mut gives me a reference
        self.total_count -= 1;
        Some(extracted_head_node.borrow_mut().event.clone_box())
    }

    /// Peek at the time of the first event.
    pub fn peek_first(&self) -> Option<SimTime> {
        // Use as_ref to get an Option<&T> so I don't take ownership, then I use .map
        // to use a closure on wrapped value that takes reference to node, access 
        // event and get SimTime creating an Option<SimTime>
        self.head.as_ref().map(|node| node.borrow().event.get_time())
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// Number of events in queue
    pub fn len(&self) -> usize {
        self.total_count
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.head = None;
        self.tail = None;
        self.total_count = 0;
    }

    /// Remove a specific event given its NodeHandle. O(1)
    pub fn remove(&mut self, node_handle: NodeHandle) -> () {

        // Cloning the Options around next and previous nodes
        let prev_weak_node_wrapper = node_handle.borrow_mut().prev.clone();
        let next_node_wrapper = node_handle.borrow_mut().next.clone();

        // Upgrade weak prev to strong Rc for easier handling
        let prev_node_wrapper = prev_weak_node_wrapper.clone().and_then(|w| w.upgrade());

        // If next node exists (i.e. Option is not None) then its previous pointer must
        // point to the Option arounf previous node (which must be cloned)
        if let Some(next_node) = next_node_wrapper.clone() {
            next_node.borrow_mut().prev = prev_weak_node_wrapper.clone();
        } 
        // If there is no next node this was the tail, so we have to update the tail
        else {
            self.tail = prev_node_wrapper.clone();
        }

        // If previous node exists (i.e. Option is not None and weak is upgradable) then
        // its next pointer must point to the OPtion around next node (it has to be cloned)
        if let Some(prev_node) = prev_node_wrapper {
            prev_node.borrow_mut().next = next_node_wrapper.clone();
        } 
        // If previous node doesn't exists this was the head, so we have to update head
        else {
            self.head = next_node_wrapper.clone();
        }

        self.total_count -= 1;
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::event::BasicEvent;

    #[test]
    fn test_insert_and_pop() {
        let mut queue = PriorityQueueDLL::new();

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
        let mut queue = PriorityQueueDLL::new();

        queue.insert(Box::new(BasicEvent::new(20))).unwrap();
        queue.insert(Box::new(BasicEvent::new(10))).unwrap();

        assert_eq!(queue.peek_first(), Some(10));
        assert_eq!(queue.len(), 2);

        queue.pop_first();
        assert_eq!(queue.peek_first(), Some(20));
    }

    #[test]
    fn test_empty_queue() {
        let mut queue: PriorityQueueDLL = PriorityQueueDLL::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.pop_first().map(|e| e.get_time()), None);
    }

    #[test]
    fn test_negative_time_rejected() {
        let mut queue = PriorityQueueDLL::new();
        let event = BasicEvent::new(-1);
        let result = queue.insert(Box::new(event));
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_events_same_time() {
        let mut queue = PriorityQueueDLL::new();

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
        let mut queue = PriorityQueueDLL::new();

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
        let mut queue = PriorityQueueDLL::new();

        let handle_first = queue.insert(Box::new(BasicEvent::new(5))).unwrap();
        let _handle_second = queue.insert(Box::new(BasicEvent::new(10))).unwrap();

        assert_eq!(queue.peek_first(), Some(5));
        queue.remove(handle_first);
        assert_eq!(queue.peek_first(), Some(10));
    }

    #[test]
    fn test_remove_tail() {
        let mut queue = PriorityQueueDLL::new();

        let _handle_first = queue.insert(Box::new(BasicEvent::new(5))).unwrap();
        let handle_last = queue.insert(Box::new(BasicEvent::new(20))).unwrap();

        assert_eq!(queue.len(), 2);
        queue.remove(handle_last);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(5));
    }

    #[test]
    fn test_clear() {
        let mut queue = PriorityQueueDLL::new();

        queue.insert(Box::new(BasicEvent::new(10))).unwrap();
        queue.insert(Box::new(BasicEvent::new(20))).unwrap();

        assert_eq!(queue.len(), 2);
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_insertion_order_preserved() {
        let mut queue = PriorityQueueDLL::new();

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
