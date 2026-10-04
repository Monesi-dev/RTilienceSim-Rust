use crate::base::sim_time::SimTime;
use super::trait_def::RandomVariable;

/// RV that has only one value
#[derive(Clone, Debug)]
pub struct Deterministic {
    value: SimTime,
}

impl Deterministic {
    pub fn new(value: SimTime) -> Self {
        Deterministic { value }
    }
}

impl RandomVariable for Deterministic {
    fn sample(&mut self) -> SimTime {
        self.value
    }

    fn clone_box(&self) -> Box<dyn RandomVariable> {
        Box::new(self.clone())
    }

    fn min(&self) -> Option<SimTime> {
        Some(self.value)
    }

    fn max(&self) -> Option<SimTime> {
        Some(self.value)
    }

    fn percentile(&self, _p: f64) -> Option<SimTime> {
        Some(self.value)
    }

    fn describe(&self) -> String {
        format!("Deterministic({})", self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_always_same() {
        let mut d = Deterministic::new(42);
        assert_eq!(d.sample(), 42);
        assert_eq!(d.sample(), 42);
    }
}
