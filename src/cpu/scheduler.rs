//! Trait for scheduling-priority policies, pluggable per simulator via a generic
//! parameter. Unlike `EventPriorities<T>` (which tie-breaks event *types* sharing the
//! same simulation time), this ranks *jobs* for the ready-queue/preemption decision -
//! fixed-priority and EDF are two policies over the same job-selection mechanism.

use crate::base::prelude::SimTime;

/// Maps a task's static priority and a job's absolute deadline to the numeric
/// scheduling priority used to order the ready queue (lower value is scheduled
/// first, consistent with every simulator's `priority < running_job_priority`
/// convention). Implementations are expected to be zero-sized marker types: the
/// mapping is a fixed, compile-time policy rather than runtime state, so `schedule`
/// is an associated function instead of a `&self` method.
pub trait Scheduler {
    fn schedule(priority: u64, deadline: SimTime) -> u64;
}

/// Fixed-priority scheduling: the job's priority is simply the task's static
/// priority, unaffected by its deadline.
#[derive(Debug, Clone, Copy)]
pub struct StaticPriority;

impl Scheduler for StaticPriority {
    fn schedule(priority: u64, _deadline: SimTime) -> u64 {
        priority
    }
}

/// Earliest Deadline First: the job's priority is its own absolute deadline, so an
/// earlier deadline always sorts first under the "lower value wins" convention.
#[derive(Debug, Clone, Copy)]
pub struct EDF;

impl Scheduler for EDF {
    fn schedule(_priority: u64, deadline: SimTime) -> u64 {
        deadline as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_priority_returns_priority_unchanged() {
        assert_eq!(StaticPriority::schedule(5, 100), 5);
        assert_eq!(StaticPriority::schedule(5, 0), 5);
    }

    #[test]
    fn test_edf_returns_deadline() {
        assert_eq!(EDF::schedule(5, 100), 100);
        assert_eq!(EDF::schedule(1, 50), 50);
    }
}
