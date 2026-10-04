use crate::base::sim_time::SimTime;
use crate::base::priority_queue::Event;

/// Base event type for simulation
#[derive(Debug, Clone)]
pub struct BasicEvent {
    pub time: SimTime,
    id: u64,
}

impl BasicEvent {
    pub fn new(time: SimTime) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        BasicEvent { time, id }
    }
}

impl Event for BasicEvent {
    fn get_time(&self) -> SimTime {
        self.time
    }

    fn set_time(&mut self, time: SimTime) {
        self.time = time;
    }

    fn get_id(&self) -> u64 {
        self.id
    }

    fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    fn doit(&mut self) -> (bool, Vec<Box<dyn Event>>) {
        (true, vec![])  // Delete this event, no follow-ups
    }

    fn clone_box(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_event_creation() {
        let event = BasicEvent::new(100);
        assert_eq!(event.time, 100);
    }

    #[test]
    fn test_basic_event_queueable() {
        let mut event = BasicEvent::new(50);
        assert_eq!(event.get_time(), 50);

        event.set_time(75);
        assert_eq!(event.get_time(), 75);
    }

    #[test]
    fn test_basic_event_doit() {
        let mut event = BasicEvent::new(100);
        let (should_delete, follow_ups) = event.doit();
        assert!(should_delete);  // Should be deleted
        assert!(follow_ups.is_empty());  // No follow-ups
    }
}
