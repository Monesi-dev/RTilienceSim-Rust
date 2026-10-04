use crate::base::sim_time::SimTime;
use std::fmt::Debug;

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
