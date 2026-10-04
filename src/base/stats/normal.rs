use crate::base::sim_time::SimTime;
use super::trait_def::RandomVariable;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, Normal as NormalDist};

/// Normal distribution with mean and standard deviation specified
#[derive(Clone, Debug)]
pub struct Normal {
    dist: NormalDist<f64>,
    mean: SimTime,
    stddev: SimTime,
    rng: StdRng,
}

impl Normal {
    pub fn new(mean: SimTime, stddev: SimTime, seed: u64) -> Self {
        let dist = NormalDist::new(mean as f64, stddev as f64)
            .expect("invalid normal distribution parameters");
        Normal {
            dist,
            mean,
            stddev,
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl RandomVariable for Normal {
    fn sample(&mut self) -> SimTime {
        let value = self.dist.sample(&mut self.rng);
        value.round() as SimTime
    }

    fn clone_box(&self) -> Box<dyn RandomVariable> {
        Box::new(self.clone())
    }

    fn describe(&self) -> String {
        format!("Normal({}, {})", self.mean, self.stddev)
    }
}
