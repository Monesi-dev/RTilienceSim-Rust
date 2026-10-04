use crate::base::sim_time::SimTime;
use super::trait_def::RandomVariable;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, StudentT};

/// Student's t-distribution with given degrees of freedom and scale
#[derive(Clone, Debug)]
pub struct TStudent {
    dist: StudentT<f64>,
    mean: SimTime,
    scale: f64,
    rng: StdRng,
}

impl TStudent {
    pub fn new(df: f64, mean: SimTime, scale: f64, seed: u64) -> Self {
        let dist = StudentT::new(df).expect("invalid student t distribution parameters");
        TStudent {
            dist,
            mean,
            scale,
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl RandomVariable for TStudent {
    fn sample(&mut self) -> SimTime {
        let value = self.dist.sample(&mut self.rng);
        (self.mean as f64 + self.scale * value).round() as SimTime
    }

    fn clone_box(&self) -> Box<dyn RandomVariable> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tstudent_samples() {
        let mut ts = TStudent::new(5.0, 0, 1.0, 42);
        for _ in 0..100 {
            let s = ts.sample();
            let _ = s;
        }
    }

    #[test]
    fn tstudent_reproducible() {
        let mut ts1 = TStudent::new(5.0, 0, 1.0, 42);
        let mut ts2 = TStudent::new(5.0, 0, 1.0, 42);

        let s1 = ts1.sample();
        let s2 = ts2.sample();
        assert_eq!(s1, s2);
    }

    #[test]
    fn tstudent_with_params() {
        let mut ts = TStudent::new(5.0, 10, 2.0, 42);
        for _ in 0..100 {
            let s = ts.sample();
            assert!(s >= -1000);
        }
    }
}
