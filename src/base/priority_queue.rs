use crate::base::sim_time::SimTime;
use std::collections::BTreeMap;
use std::fmt::Debug;

// Event is defined as a trait, so it is basically an interface with getter and setter
// for time, a getter for ID and the doit method that encapsulates the processing of 
// the event, it returns a vector with new events to schedule. 
// @NB: We are destroying and building a message which could potentially just be 
// reinserted, but this is a design choice to avoid future problems.
// @NB: id was added because when I had to delete a random event I did not know how to 
// do it without using the pointer (which is unsafe rust)
pub trait Event: Debug {
    fn get_time(&self) -> SimTime;
    fn set_time(&mut self, time: SimTime);
    fn get_id(&self) -> u64;
    fn set_id(&mut self, id: u64);
    fn doit(&mut self) -> Vec<Box<dyn Event>>;
    fn clone_box(&self) -> Box<dyn Event>;
}

// Priority queue using BTreeMap: we can access a key in logN, we can access the first 
// key in logN, and we can remove a key in logN.
// Since there could be 1+ events at the same time, we store a vector of events 
// @NB: I have used a dynamic trait for Vector, it is dispatched dynamically (i.e. at 
// runtime we look at the type of event and use its methods) it is slower than static 
// dispatching (i.e. at compile time we know the type of event and use its methods) but
// I think it is the only way to have a priority queue with different types of events.
#[derive(Debug)]
pub struct PriorityQueue {
    items: BTreeMap<SimTime, Vec<Box<dyn Event>>>,
    total_count: usize,
}

impl PriorityQueue {
    pub fn new() -> Self {
        PriorityQueue {
            items: BTreeMap::new(),
            total_count: 0,
        }
    }

    /// Insert an event looking at it time for the order (O(log n))
    pub fn insert(&mut self, entity: Box<dyn Event>) -> Result<(), String> {
        
        // Read time of message, if negative return error
        let time = entity.get_time();
        if time < 0 {
            return Err("Event time cannot be negative".to_string());
        }

        // Get vector of messages at time (if none create an empty vector) and then
        // add the message to the vector (and update message number)
        let messages = self.items.entry(time).or_insert_with(Vec::new);
        messages.push(entity);
        self.total_count += 1;

        // @NB You cannot return () but need Ok(()) because Result<A, B> means
        // that it's Ok(A) or Err(B)
        Ok(())
    }

    /// Extract first (minimum time) event (O(log n))
    pub fn pop_first(&mut self) -> Option<Box<dyn Event>> {
        
        // BTreeMap.keys() gives me an iterator of Option<&SimTime>, I use .next() to
        // get the first reference key and then .copied() to get SimTime
        let mut keys_iterator = self.items.keys();
        let first_key_ref = keys_iterator.next();
        let first_key = first_key_ref.copied();

        // If queue empty then return None 
        let first_key = first_key?;

        // Get list of entities at that time, if no list return None
        let entities = self.items.get_mut(&first_key)?;

        // This should never happen, but just in case, we check
        if entities.is_empty() {
            println!("Warning: Found empty entity list for time {}", first_key);
            return None;
        }

        // Remove the first entity from the list
        let entity = entities.remove(0);
        self.total_count -= 1;

        // Clean up empty time slot
        if entities.is_empty() {
            self.items.remove(&first_key);
        }

        Some(entity)
    }

    // Peek at the first time (I think it may be needed for the simulation) 
    pub fn peek_first(&self) -> Option<SimTime> {
        self.items.keys().next().copied()
    }

    // Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    // Number of entities in queue
    pub fn len(&self) -> usize {
        self.total_count
    }

    // Remove all entities
    pub fn clear(&mut self) {
        self.items.clear();
        self.total_count = 0;
    }

    // Remove a specific event from the queue, I use time to get the bucket and then
    // I look at the ID
    pub fn remove(&mut self, event: Box<dyn Event>) -> bool {

        // Get time and id
        let time = event.get_time();
        let event_id = event.get_id();

        // This shouldn't happen
        let Some(entities) = self.items.get_mut(&time) else {
            return false;
        };
        
        for (idx, entity) in entities.iter().enumerate() {
            if entity.get_id() == event_id {
                entities.remove(idx);
                self.total_count -= 1;

                // Clean up empty bucket
                if entities.is_empty() {
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

    // Defining an Event to test it, when I implement the others I can use them to test
    #[derive(Debug, Clone)]
    struct TestEvent {
        time: SimTime,
        event_id: u64,
    }
    
    impl TestEvent {
        // @NB: I used those std::sync::atomic::AtomicU64 to increment a static variable
        // the compiler was giving me problems about mutable variables 
        fn new(time: SimTime) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let event_id = COUNTER.fetch_add(1, Ordering::SeqCst);
            TestEvent { time, event_id }
        }
    }

    impl Event for TestEvent {
        fn get_time(&self) -> SimTime {
            self.time
        }

        fn set_time(&mut self, time: SimTime) {
            self.time = time;
        }

        fn get_id(&self) -> u64 {
            self.event_id
        }

        fn set_id(&mut self, id: u64) {
            self.event_id = id;
        }

        fn doit(&mut self) -> Vec<Box<dyn Event>> {
            vec![]
        }

        fn clone_box(&self) -> Box<dyn Event> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn test_insert_and_pop() {
        let mut queue = PriorityQueue::new();

        // Insert events, if they fail (return None) it should panic so I use unwrap()
        queue.insert(Box::new(TestEvent::new(10))).unwrap();
        queue.insert(Box::new(TestEvent::new(5))).unwrap();
        queue.insert(Box::new(TestEvent::new(15))).unwrap();

        // Checking that the wrapper is working
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(5));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(15));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), None);
    }

    #[test]
    fn test_peek_first() {
        let mut queue = PriorityQueue::new();

        // Insert events, if they fail (return None) it should panic so I use unwrap()
        queue.insert(Box::new(TestEvent::new(20))).unwrap();
        queue.insert(Box::new(TestEvent::new(10))).unwrap();

        // Peek first, it should not remove
        assert_eq!(queue.peek_first(), Some(10));
        assert_eq!(queue.len(), 2); 

        // Pop first we should peek the second
        queue.pop_first();
        assert_eq!(queue.peek_first(), Some(20));
    }

    #[test]
    fn test_empty_queue() {
        let mut queue: PriorityQueue = PriorityQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.pop_first().map(|e| e.get_time()), None);
    }

    #[test]
    fn test_negative_time_rejected() {
        let mut queue = PriorityQueue::new();
        let event = TestEvent::new(-1);
        assert_eq!(queue.insert(Box::new(event)), Err("Event time cannot be negative".to_string()));
    }

    #[test]
    fn test_multiple_events_same_time() {
        let mut queue = PriorityQueue::new();

        // Insert 1+ events at the same time
        queue.insert(Box::new(TestEvent::new(10))).unwrap();
        queue.insert(Box::new(TestEvent::new(10))).unwrap();
        queue.insert(Box::new(TestEvent::new(10))).unwrap();
        queue.insert(Box::new(TestEvent::new(5))).unwrap();

        // Should extract time 5 first, then time 10
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(5));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert!(queue.pop_first().is_none());
    }

    #[test]
    fn test_remove_event() {
        let mut queue = PriorityQueue::new();

        // Insert events
        let event1 = TestEvent::new(10);
        let event1_id = event1.get_id();
        queue.insert(Box::new(event1)).unwrap();
        queue.insert(Box::new(TestEvent::new(10))).unwrap();
        queue.insert(Box::new(TestEvent::new(20))).unwrap();

        assert_eq!(queue.len(), 3);

        // Create search key with same time and ID
        let mut search = Box::new(TestEvent::new(99));
        search.time = 10;
        search.event_id = event1_id;

        // Remove event1
        let removed = queue.remove(search);
        assert!(removed);
        assert_eq!(queue.len(), 2);

        // Verify queue structure
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(10));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), Some(20));
        assert_eq!(queue.pop_first().map(|e| e.get_time()), None);
    }
}

// Integration tests with sporadic and periodic events
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::base::sporadic_event::SporadicEvent;
    use crate::base::periodic_event::PeriodicEvent;
    use crate::base::random_variable::{Uniform, Normal, Exponential, Poisson};

    #[test]
    fn test_event_list_with_distributions_and_deletion() {

        // Test priority queue creating 5 events (Uniform, Normal, Exponential, Poisson
        // and Periodic). Then extract and re-insert 50 times. Then delete last element
        // popped (which will be in a random position). Then extract and re-insert 50 times
        const SEED: u64 = 12345;
        const EXTRACT_REINSERT_ROUNDS: usize = 50;

        let extracted_times = run_event_list_scenario(SEED, EXTRACT_REINSERT_ROUNDS);
        let expected_length = EXTRACT_REINSERT_ROUNDS + 1 + EXTRACT_REINSERT_ROUNDS;
        assert_eq!(extracted_times.len(), expected_length);

        // Verify events were extracted in non-decreasing time order (priority queue property)
        assert!(is_monotonically_increasing(&extracted_times));

        // Verify that simulations is reproducible
        let extracted_times_rep = run_event_list_scenario(SEED, EXTRACT_REINSERT_ROUNDS);
        assert_eq!(extracted_times, extracted_times_rep);
    }

    fn run_event_list_scenario(seed: u64, extract_reinsert_rounds: usize) -> Vec<SimTime> {
        

        let mut queue = PriorityQueue::new();
        let mut extracted_times: Vec<SimTime> = Vec::new();

        // Create Events
        let sporadic_uniform = SporadicEvent::new(
            0, Box::new(Uniform::new(1, 10, seed))
        );
        let sporadic_normal = SporadicEvent::new(
            5, Box::new(Normal::new(10, 2, seed))
        );
        let sporadic_exponential = SporadicEvent::new(
            10, Box::new(Exponential::new(0.1, 5.0, seed)),
        );
        let sporadic_poisson = SporadicEvent::new(
            15, Box::new(Poisson::new(5.0, seed))
        );
        let periodic = PeriodicEvent::new(2, 20);

        // Insert messages
        queue.insert(Box::new(sporadic_uniform)).unwrap();
        queue.insert(Box::new(sporadic_normal)).unwrap();
        queue.insert(Box::new(sporadic_exponential)).unwrap();
        queue.insert(Box::new(sporadic_poisson)).unwrap();
        queue.insert(Box::new(periodic)).unwrap();


        // Extract and re-insert 50 times
        for _ in 0..extract_reinsert_rounds {
            let mut event = queue.pop_first().unwrap();
            extracted_times.push(event.get_time());

            let new_events = event.doit();
            for new_event in new_events {
                queue.insert(new_event).unwrap();
            }
        }

        // Extract Event, insert new event, update time of Event to new Event (so they
        // will have same time and id) and use it to delete newly inserted event
        let mut event = queue.pop_first().unwrap();
        extracted_times.push(event.get_time());

        let new_events = event.doit();
        event.set_time(new_events[0].get_time());
        for new_event in new_events {
            queue.insert(new_event).unwrap();
        }

        assert_eq!(queue.remove(event), false);
        assert_eq!(queue.len(), 5);

        // Extract and re-insert 50 more times
        for _ in 0..extract_reinsert_rounds {
            let mut event = queue.pop_first().unwrap();
            extracted_times.push(event.get_time());

            let new_events = event.doit();
            for new_event in new_events {
                queue.insert(new_event).unwrap();
            }
        }

        extracted_times
    }

    // Verifies that vector of times is not-decreasing
    fn is_monotonically_increasing(times: &[SimTime]) -> bool {
        times.windows(2).all(|w| w[0] <= w[1])
    }
}
