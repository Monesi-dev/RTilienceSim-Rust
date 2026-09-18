use crate::base::sim_time::SimTime;
use crate::base::priority_queue::{PriorityQueue, Event};

/// Event list that manages simulation events
/// Embeds a PriorityQueue and adds event execution capabilities
#[derive(Debug)]
pub struct EventList {
    queue: PriorityQueue,
    last_time: SimTime,
}

impl EventList {

    pub fn new() -> Self {
        EventList {
            queue: PriorityQueue::new(),
            last_time: SimTime::MAX,
        }
    }

    // Set max time for events in list
    pub fn set_last_time(&mut self, time: SimTime) {
        self.last_time = time;
    }

    // Inserts an event in list, if time is above threshold returns Err
    pub fn insert(&mut self, entity: Box<dyn Event>) -> Result<(), String> {
        let time = entity.get_time();

        if time >= self.last_time {
            return Err(format!(
                "Event time {} exceeds last_time {}",
                time, self.last_time
            ));
        }

        // @NB: this is the return value
        self.queue.insert(entity)
    }

    // Runs a simulation until condition is met or until events are finished
    // A simulation is extract first event, execute it, insert new events created
    pub fn run_conditionally<F>(&mut self, mut condition: F)
    where
        F: FnMut() -> bool,
    {
        loop {
            // Pop first event from queue, if queue is empty, break
            let Some(mut event) = self.queue.pop_first() else {
                break;  // Queue empty
            };

            // Event processing happens and the new events to insert are returned
            let follow_ups = event.doit();

            // Insert new events
            // @NB here we ignore Result<> with let _ = (otherwise it gives a warning) 
            // but in the future it can be used to log
            for follow_up in follow_ups {
                let _ = self.insert(follow_up);
            }

            // Check custom condition to continue (it's !f() in C++)
            if !condition() {
                break;
            }
        }
    }

    // Runs simulation until no more events
    pub fn run(&mut self, end_time: SimTime) {
        self.last_time = end_time;
        self.run_conditionally(|| true);
    }

    // Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    // Peek at next event time
    pub fn peek_first(&self) -> Option<SimTime> {
        self.queue.peek_first()
    }

    // Number of events in queue
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    // Clear all events
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::event::BasicEvent;

    #[test]
    fn test_eventlist_creation() {
        let list = EventList::new();
        assert_eq!(list.is_empty(), true);
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_eventlist_insert() {
        let mut list = EventList::new();

        let event1: Box<dyn Event> = Box::new(BasicEvent::new(10));
        let event2: Box<dyn Event> = Box::new(BasicEvent::new(5));

        assert_eq!(list.insert(event1), Ok(()));
        assert_eq!(list.insert(event2), Ok(()));
        assert_eq!(list.len(), 2);
        assert_eq!(list.peek_first(), Some(5));  // Sorted order
    }

    #[test]
    fn test_eventlist_reject_out_of_bounds() {
        let mut list = EventList::new();
        list.set_last_time(100);

        let event: Box<dyn Event> = Box::new(BasicEvent::new(150));

        assert_eq!(list.insert(event), Err("Event time 150 exceeds last_time 100".to_string()));
        assert_eq!(list.is_empty(), true);
    }

    #[test]
    fn test_eventlist_accept_in_bounds() {
        let mut list = EventList::new();
        list.set_last_time(100);

        let event: Box<dyn Event> = Box::new(BasicEvent::new(50));

        let result = list.insert(event);
        assert_eq!(result, Ok(()));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_eventlist_sorted_extraction() {
        let mut list = EventList::new();

        list.insert(Box::new(BasicEvent::new(30))).unwrap();
        list.insert(Box::new(BasicEvent::new(10))).unwrap();
        list.insert(Box::new(BasicEvent::new(20))).unwrap();

        // Extract in sorted order
        assert_eq!(list.peek_first(), Some(10));
        let e1 = list.queue.pop_first();
        assert_eq!(e1.is_some(), true);

        assert_eq!(list.peek_first(), Some(20));
        let e2 = list.queue.pop_first();
        assert_eq!(e2.is_some(), true);

        assert_eq!(list.peek_first(), Some(30));
        let e3 = list.queue.pop_first();
        assert_eq!(e3.is_some(), true);

        assert_eq!(list.is_empty(), true);
    }

    #[test]
    fn test_eventlist_with_periodic_events() {
        use crate::base::periodic_event::PeriodicEvent;

        let mut list = EventList::new();
        list.set_last_time(50);

        // Insert a periodic event that runs at  5, 15, 25, 35, 45
        list.insert(Box::new(PeriodicEvent::new(5, 10))).unwrap();

        assert_eq!(list.len(), 1);
        assert_eq!(list.peek_first(), Some(5));

        // Run simulation for 5 iterations
        let mut count = 0;
        list.run_conditionally(|| {
            count += 1;
            count < 5
        });

        // After 5 iterations, all periodic events should be exhausted
        assert_eq!(list.is_empty(), true);
    }

}
