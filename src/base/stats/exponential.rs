use crate::base::sim_time::SimTime;
use super::trait_def::RandomVariable;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, Exp};

/// Exponential distribution with given rate and scale factor
#[derive(Clone, Debug)]
pub struct Exponential {
    dist: Exp<f64>,
    rate: f64,
    scale: f64,
    rng: StdRng,
}

impl Exponential {
    pub fn new(rate: f64, scale: f64, seed: u64) -> Self {
        // rand_distr uses lambda = 1/mean, so we use rate
        let dist = Exp::new(rate).expect("invalid exponential distribution parameters");
        Exponential {
            dist,
            rate,
            scale,
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl RandomVariable for Exponential {
    fn sample(&mut self) -> SimTime {
        let value = self.dist.sample(&mut self.rng);
        (value * self.scale).round() as SimTime
    }

    fn clone_box(&self) -> Box<dyn RandomVariable> {
        Box::new(self.clone())
    }

    fn min(&self) -> Option<SimTime> {
        Some(0)
    }

    fn describe(&self) -> String {
        format!("Exponential({}, scale={})", self.rate, self.scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_positive() {
        let mut e = Exponential::new(0.1, 1.0, 42);
        for _ in 0..100 {
            assert!(e.sample() >= 0);
        }
    }
}
