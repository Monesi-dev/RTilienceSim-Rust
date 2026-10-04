use crate::base::sim_time::SimTime;
use super::trait_def::RandomVariable;
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Uniform distribution on [min, max]
#[derive(Clone, Debug)]
pub struct Uniform {
    min: SimTime,
    max: SimTime,
    rng: StdRng,
}

impl Uniform {
    pub fn new(min: SimTime, max: SimTime, seed: u64) -> Self {
        Uniform {
            min,
            max,
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl RandomVariable for Uniform {
    fn sample(&mut self) -> SimTime {
        self.rng.gen_range(self.min..=self.max)
    }

    fn clone_box(&self) -> Box<dyn RandomVariable> {
        Box::new(self.clone())
    }

    fn min(&self) -> Option<SimTime> {
        Some(self.min)
    }

    fn max(&self) -> Option<SimTime> {
        Some(self.max)
    }

    fn percentile(&self, p: f64) -> Option<SimTime> {
        let var = p * ((self.max - self.min) as f64);
        Some(var as SimTime + self.min)
    }

    fn describe(&self) -> String {
        format!("Uniform({}, {})", self.min, self.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_in_range() {
        let mut u = Uniform::new(10, 20, 42);
        for _ in 0..100 {
            let s = u.sample();
            assert!(s >= 10 && s <= 20);
        }
    }

    #[test]
    fn uniform_reproducible() {
        let mut u1 = Uniform::new(10, 20, 42);
        let mut u2 = Uniform::new(10, 20, 42);

        for _ in 0..100 {
            assert_eq!(u1.sample(), u2.sample());
        }
    }
}
