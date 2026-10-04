use crate::base::sim_time::SimTime;
use super::trait_def::RandomVariable;
use std::fs::File;
use std::io::{BufRead, BufReader};
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;

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
}
