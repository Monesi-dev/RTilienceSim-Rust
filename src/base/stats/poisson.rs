use crate::base::sim_time::SimTime;
use super::trait_def::RandomVariable;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, Poisson as PoissonDist};

/// Poisson distribution with given mean
#[derive(Clone, Debug)]
pub struct Poisson {
    dist: PoissonDist<f64>,
    mean: f64,
    rng: StdRng,
}

impl Poisson {
    pub fn new(mean: f64, seed: u64) -> Self {
        let dist = PoissonDist::new(mean).expect("invalid poisson distribution parameters");
        Poisson {
            dist,
            mean,
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl RandomVariable for Poisson {
    fn sample(&mut self) -> SimTime {
        let value = self.dist.sample(&mut self.rng);
        value as SimTime
    }

    fn clone_box(&self) -> Box<dyn RandomVariable> {
        Box::new(self.clone())
    }

    fn min(&self) -> Option<SimTime> {
        Some(0)
    }

    fn describe(&self) -> String {
        format!("Poisson({})", self.mean)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisson_integer() {
        let mut p = Poisson::new(5.0, 42);
        for _ in 0..100 {
            let s = p.sample();
            assert!(s >= 0);
        }
    }
}
