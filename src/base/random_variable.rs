use crate::base::sim_time::SimTime;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, BufReader};
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, Normal as NormalDist, Exp, Poisson as PoissonDist, StudentT};

/// Trait for a RV from which you sample values
pub trait RandomVariable: Debug {

    /// Sample value from RV
    fn sample(&mut self) -> SimTime;

    /// Returns a deep copy of the RV (the sequence of next sampled values is identical)
    fn clone_box(&self) -> Box<dyn RandomVariable>;

    /// Get min value that can be sample and None if there is no lower bound
    /// @NB: This could be removed as it has been kept from C++ lib
    fn min(&self) -> Option<SimTime> {
        None
    }

    /// Get max value that can be sampled and None if there is no upper bound
    // @NB: This could be removed as it has been kept from C++ lib
    fn max(&self) -> Option<SimTime> {
        None
    }

    /// Return value v' so that a value <= v' is sampled is equal to _p
    // @NB: This could be removed as it has been kept from C++ lib
    fn percentile(&self, _p: f64) -> Option<SimTime> {
        None
    }

    /// Return a string with debug information on RV
    /// @NB: This could be removed but it may be useful for debugging
    fn describe(&self) -> String {
        format!("{:?}", self)
    }
}

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

/// Deterministic values read sequentially from file
#[derive(Clone, Debug)]
pub struct DetVar {
    values: Vec<SimTime>,
    pos: usize,
}

impl DetVar {
    pub fn from_file(path: &str) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut values: Vec<SimTime> = Vec::new();
        for line in reader.lines() {
            let val = line
                .expect("Failed to read line")
                .trim()
                .parse::<SimTime>()
                .expect("Number not parsed");
            values.push(val);
        }

        Ok(DetVar {
            values: values,
            pos: 0,
        })
    }

    pub fn from_vec(values: Vec<SimTime>) -> Self {
        DetVar { values, pos: 0 }
    }
}

impl RandomVariable for DetVar {
    fn sample(&mut self) -> SimTime {
        if self.values.is_empty() {
            return -1;
        }

        if self.pos >= self.values.len() {
            -1
        } else {
            let val = self.values[self.pos];
            self.pos += 1;
            val
        }
    }

    fn clone_box(&self) -> Box<dyn RandomVariable> {
        Box::new(self.clone())
    }

    fn min(&self) -> Option<SimTime> {
        self.values.iter().cloned().min()
    }

    fn max(&self) -> Option<SimTime> {
        self.values.iter().cloned().max()
    }

    fn describe(&self) -> String {
        format!("DetVar({} values)", self.values.len())
    }
}

/// Custom discrete/continuous distribution from file or array
/// Stores values and their cumulative probabilities
#[derive(Clone, Debug)]
pub struct FileVar {
    values: Vec<SimTime>,
    cumulative_probs: Vec<f64>,
    continuous: bool,
    rng: StdRng,
}

impl FileVar {
    /// Create from arrays of values and cumulative probabilities
    pub fn new(values: Vec<SimTime>, cumulative_probs: Vec<f64>, continuous: bool, seed: u64) -> Self {

        // Verify that lengths matches and they are not empty
        assert_eq!(values.len(), cumulative_probs.len(), "values and probs must have same length");
        assert!(!cumulative_probs.is_empty(), "must have at least one value");

        // Verify cumulative probabilities are in [0, 1] and increasing
        let mut prev = 0.0;
        for &p in &cumulative_probs {
            assert!(p >= prev && p <= 1.0, "cumulative probs must be increasing and in [0,1]");
            prev = p;
        }

        FileVar {
            values,
            cumulative_probs,
            continuous,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Create from probability array (index i has probability probs[i])
    pub fn from_probabilities(probs: Vec<f64>, seed: u64) -> Self {
        let mut cumulative = Vec::with_capacity(probs.len());
        let mut sum = 0.0;
        let total: f64 = probs.iter().sum();

        for p in probs {
            sum += p / total;
            if (1.0 - sum).abs() < 1e-7 {
                sum = 1.0;
            }
            cumulative.push(sum);
        }

        let values: Vec<SimTime> = (0..cumulative.len()).map(|i| i as SimTime).collect();
        FileVar {
            values,
            cumulative_probs: cumulative,
            continuous: true,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Create a geometric distribution with parameter p
    pub fn geometric(p: f64, seed: u64) -> Self {

        let mut values = Vec::new();
        let mut cumulative_probs = Vec::new();
        let mut prod = p;
        let mut sum = p;
        let max_values = 1024;

        values.push(1);
        cumulative_probs.push(p);

        while sum < 1.0 && values.len() < max_values {
            prod *= 1.0 - p;
            sum += prod;
            if (1.0 - sum).abs() < 1e-7 {
                sum = 1.0;
            }
            values.push(values.len() as SimTime + 1);
            cumulative_probs.push(sum);
        }

        FileVar {
            values,
            cumulative_probs,
            continuous: false,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Load from file with format: "value probability" per line
    pub fn from_file(path: &str, continuous: bool, seed: u64) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut values = Vec::new();
        let mut cumulative_probs = Vec::new();
        let mut current_cumulative = 0.0;

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let val: SimTime = parts[0]
                .parse()
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error"))?;
            let prob: f64 = parts[1]
                .parse()
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error"))?;

            values.push(val);
            current_cumulative += prob;
            if current_cumulative > 1.0 {
                current_cumulative = 1.0;
            }
            cumulative_probs.push(current_cumulative);
        }

        Ok(FileVar {
            values,
            cumulative_probs,
            continuous,
            rng: StdRng::seed_from_u64(seed),
        })
    }

    fn binary_search_prob(&self, r: f64) -> usize {
        self.cumulative_probs
            .binary_search_by(|&p| {
                if p < r {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            })
            .unwrap_or_else(|i| i)
            .min(self.cumulative_probs.len() - 1)
    }
}


impl RandomVariable for FileVar {
    fn sample(&mut self) -> SimTime {
        if self.values.is_empty() || self.cumulative_probs.is_empty() {
            return -1;
        }

        if (1.0 - self.cumulative_probs[self.cumulative_probs.len() - 1]).abs() > 1e-6 {
            eprintln!("Final cum_prob not 1");
            return -1;
        }

        let r: f64 = self.rng.gen_range(0.0..1.0);
        let i = self.binary_search_prob(r);

        let mut res = self.values[i] as f64;

        if self.continuous && i > 0 && self.cumulative_probs[i - 1] > 0.0 {
            let frac = (self.cumulative_probs[i] - r) / (self.cumulative_probs[i] - self.cumulative_probs[i - 1]);
            res -= frac * (self.values[i] as f64 - self.values[i - 1] as f64);
        }

        res.round() as SimTime
    }

    fn clone_box(&self) -> Box<dyn RandomVariable> {
        Box::new(self.clone())
    }

    fn min(&self) -> Option<SimTime> {
        self.values.iter().cloned().min()
    }

    fn max(&self) -> Option<SimTime> {
        self.values.iter().cloned().max()
    }

    fn percentile(&self, p: f64) -> Option<SimTime> {
        let i = self.binary_search_prob(p);
        let mut res = self.values[i] as f64;

        if i > 0 {
            let frac = (self.cumulative_probs[i] - p) / (self.cumulative_probs[i] - self.cumulative_probs[i - 1]);
            res -= frac * (self.values[i] as f64 - self.values[i - 1] as f64);
        }

        Some(res.round() as SimTime)
    }

    fn describe(&self) -> String {
        format!("FileVar({} points, continuous={})", self.values.len(), self.continuous)
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

    #[test]
    fn exponential_positive() {
        let mut e = Exponential::new(0.1, 1.0, 42);
        for _ in 0..100 {
            assert!(e.sample() >= 0);
        }
    }

    #[test]
    fn poisson_integer() {
        let mut p = Poisson::new(5.0, 42);
        for _ in 0..100 {
            let s = p.sample();
            assert!(s >= 0);
        }
    }

    #[test]
    fn detvar_sequence() {
        let mut dv = DetVar::from_vec(vec![1, 2, 3, 4, 5]);
        assert_eq!(dv.sample(), 1);
        assert_eq!(dv.sample(), 2);
        assert_eq!(dv.sample(), 3);
        assert_eq!(dv.sample(), 4);
        assert_eq!(dv.sample(), 5);
        assert_eq!(dv.sample(), -1);
    }

    #[test]
    fn filevar_sample() {
        let mut fv = FileVar::new(
            vec![10, 20, 30],
            vec![0.3, 0.7, 1.0],
            false,
            42,
        );
        for _ in 0..100 {
            let s = fv.sample();
            assert!(s == 10 || s == 20 || s == 30);
        }
    }

    #[test]
    fn filevar_cloned_preserves_sequence() {
        let mut fv1 = FileVar::new(
            vec![10, 20, 30],
            vec![0.3, 0.7, 1.0],
            false,
            42,
        );
        let mut fv2 = fv1.clone();

        // Both should produce same sequence since they have same RNG state
        let s1 = fv1.sample();
        let s2 = fv2.sample();
        assert_eq!(s1, s2);
    }

    #[test]
    fn geometric_dist() {
        let g = FileVar::geometric(0.3, 42);
        assert!(!g.values.is_empty());
        assert!(!g.cumulative_probs.is_empty());
        assert!((g.cumulative_probs[g.cumulative_probs.len() - 1] - 1.0).abs() < 1e-6);
    }

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
