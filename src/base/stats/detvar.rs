use crate::base::sim_time::SimTime;
use super::trait_def::RandomVariable;
use std::fs::File;
use std::io::{BufRead, BufReader};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
