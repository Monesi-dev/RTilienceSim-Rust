use crate::base::sim_time::SimTime;
use crate::base::priority_queue::Event;
use crate::base::random_variable::RandomVariable;

/// Event that reschedules itself incrementing time by a value sampled from a RV
#[derive(Debug)]
pub struct SporadicEvent {
    pub time: SimTime,
    pub random_var: Box<dyn RandomVariable>,
    id: u64,
}

impl Clone for SporadicEvent {
    fn clone(&self) -> Self {
        SporadicEvent {
            time: self.time,
            random_var: self.random_var.clone_box(),
            id: self.id,
        }
    }
}

impl SporadicEvent {
    pub fn new(time: SimTime, random_var: Box<dyn RandomVariable>) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        SporadicEvent {
            time,
            random_var,
            id,
        }
    }
}

impl Event for SporadicEvent {
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

    fn doit(&mut self) -> Vec<Box<dyn Event>> {
        // Sample delta time and reschedule with cloned random variable
        // @NB: I clone the RV because I cannot pass ownership as self is a reference.
        // If I wanted to move ownership I would need to make self a Box<Self> and then
        // I would make result the same value
        let delta = self.random_var.sample();
        vec![Box::new(SporadicEvent::new(
            self.time + delta,
            self.random_var.clone_box(),
        ))]
    }

    fn clone_box(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct FixedRandomVar {
        value: SimTime,
    }

    impl FixedRandomVar {
        fn new(value: SimTime) -> Self {
            FixedRandomVar { value }
        }
    }

    impl RandomVariable for FixedRandomVar {
        fn sample(&mut self) -> SimTime {
            self.value
        }

        fn clone_box(&self) -> Box<dyn RandomVariable> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn test_sporadic_event_creation() {
        let event = SporadicEvent::new(0, Box::new(FixedRandomVar::new(5)));
        assert_eq!(event.get_time(), 0);
    }

    #[test]
    fn test_sporadic_event_reschedules() {
        let mut event = SporadicEvent::new(10, Box::new(FixedRandomVar::new(5)));
        let follow_ups = event.doit();

        assert_eq!(follow_ups.len(), 1);
        assert_eq!(follow_ups[0].get_time(), 15);
    }

    #[test]
    fn test_sporadic_event_multiple_occurrences() {
        let mut times = vec![];
        let mut event: Box<dyn Event> =
            Box::new(SporadicEvent::new(0, Box::new(FixedRandomVar::new(7))));

        times.push(event.get_time());
        for _ in 0..5 {
            let follow_ups = event.doit();
            event = follow_ups.into_iter().next().unwrap();
            times.push(event.get_time());
        }

        assert_eq!(times, vec![0, 7, 14, 21, 28, 35]);
    }
}
