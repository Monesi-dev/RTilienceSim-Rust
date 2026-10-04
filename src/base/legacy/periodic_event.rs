use crate::base::sim_time::SimTime;
use crate::base::priority_queue::Event;

/// Event that reinsert itself adding a period to its time.
#[derive(Debug, Clone)]
pub struct PeriodicEvent {
    pub time: SimTime,
    pub period: SimTime,
    id: u64,
}

impl PeriodicEvent {
    pub fn new(time: SimTime, period: SimTime) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        PeriodicEvent { time, period, id }
    }
}

impl Event for PeriodicEvent {
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
        self.time += self.period;
        (false, vec![])
    }

    fn clone_box(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_periodic_event_creation() {
        let event = PeriodicEvent::new(0, 10);
        assert_eq!(event.get_time(), 0);
        assert_eq!(event.period, 10);
    }

    #[test]
    fn test_periodic_event_reschedules() {
        let mut event = PeriodicEvent::new(10, 5);
        let (should_delete, follow_ups) = event.doit();

        assert!(!should_delete);  // Should be reinserted
        assert_eq!(event.get_time(), 15);  // Event time should be updated
        assert!(follow_ups.is_empty());  // No additional follow-ups
    }

    #[test]
    fn test_periodic_event_multiple_periods() {
        let mut times = vec![];
        let mut event: Box<dyn Event> = Box::new(PeriodicEvent::new(0, 10));

        times.push(event.get_time());
        for _ in 0..6 {
            let (should_delete, follow_ups) = event.doit();
            assert!(!should_delete);  // Should not be deleted
            assert!(follow_ups.is_empty());  // No additional events
            times.push(event.get_time());
        }

        assert_eq!(times, vec![0, 10, 20, 30, 40, 50, 60]);
    }
}
