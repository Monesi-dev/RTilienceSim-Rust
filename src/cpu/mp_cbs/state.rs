//! # State Structures for Multi-Processor CBS Scheduler
//!
//! Contains the data structures that represent the state of the simulation.
//! Unlike `mp` (jobs compete directly in one global ready queue by task priority),
//! here every task belongs to a Constant Bandwidth Server. Scheduling happens on two
//! levels: inside a server its own jobs compete by deadline (intra-server EDF), and the
//! servers themselves compete for the CPUs by their own scheduling deadline, which the
//! CBS rules move around as budget is consumed and replenished.

use crate::base::prelude::{SimTime, RandomVariable, EventPriorities};
use std::collections::{BTreeSet, BinaryHeap, HashMap};
use std::cmp::Reverse;

/// Enum with Event types used in this simulation
/// `TaskTrigger` means that a task is about to spawn a job
/// `JobCompletion` means that job being executed has ended
/// `BudgetExhausted` means that the running server has consumed all of its budget
/// `EndSimulation` means that simulation has to stop
#[derive(Debug, Clone, Copy)]
pub enum EventType {
    TaskTrigger,
    JobCompletion,
    BudgetExhausted,
    EndSimulation,
}

/// Tie-breaks events sharing the same simulation time:
/// `JobCompletion` < `BudgetExhausted` < `TaskTrigger` < `EndSimulation`.
/// A job finishing is seen first so an arriving task never looks at a CPU that is about
/// to be freed, and a server that runs out of budget settles its deadline before an
/// arriving job is compared against it.
#[derive(Debug, Clone, Copy)]
pub struct EventPriorityPolicy;

impl EventPriorities<EventType> for EventPriorityPolicy {
    fn priority(event_type: &EventType) -> u32 {
        match event_type {
            EventType::JobCompletion => 0,
            EventType::BudgetExhausted => 1,
            EventType::TaskTrigger => 2,
            EventType::EndSimulation => 3,
        }
    }
}

/// Describes one task to register inside a server, passed to `add_server`.
pub struct TaskSpec {
    pub start_time: SimTime,
    pub relative_deadline: SimTime,
    pub execution_time_rng: Box<dyn RandomVariable>,
    pub interarrival_time_rng: Box<dyn RandomVariable>,
    pub store_response_times: bool,
}

/// This is a Task Descriptor struct, it is used to store, in the global state, everything
/// that should be remembered about a task.
/// `relative_deadline` is used to get the deadline of a spawned job, which doubles as
/// that job's priority inside its server.
/// `execution_time_rng` is used to sample execution time of spawned job.
/// `interarrival_time_rng` is used to sample time after which a new job will be spawned.
/// `server_id` is the server this task (and so all of its jobs) is served by.
#[derive(Debug)]
pub struct TaskType {
    pub relative_deadline: SimTime,
    pub execution_time_rng: Box<dyn RandomVariable>,
    pub interarrival_time_rng: Box<dyn RandomVariable>,
    pub server_id: u64,
}

/// This is a Job Descriptor struct, it is used to store, in the global state, everything
/// that should be known about a job.
/// `scheduled_on_cpu` is the time at which it was scheduled on cpu and idx of the cpu
/// if it is a running job otherwise it is None
/// `remaining_time` is the time from scheduled_on_cpu or, if None, from when it will be
/// scheduled remaining until completion
/// `deadline` is the job's absolute deadline, which is also its priority inside its server
/// `task_id` links back to the task that spawned this job, used to reach its server
#[derive(Debug)]
pub struct JobDescriptor {
    pub scheduled_on_cpu: Option<(SimTime, usize)>,
    pub remaining_time: SimTime,
    pub deadline: SimTime,
    pub task_id: u64,
}

/// This is a Server Descriptor struct, it holds one Constant Bandwidth Server: the
/// bandwidth reservation itself plus the jobs it is serving.
#[derive(Debug)]
pub struct ServerDescriptor {
    /// `Q` - the budget the server is given at every replenishment
    pub max_budget: SimTime,
    /// `T` - the server period, every postponement pushes the deadline out by this much
    pub period: SimTime,
    /// `U` - the server bandwidth Q/T, cached to evaluate the recharge rule
    pub bandwidth: f64,
    /// `c` - the budget still available before the server is throttled
    pub budget: SimTime,
    /// `d` - the server's scheduling deadline, which is its priority against other servers
    pub deadline: SimTime,
    /// The job this server is currently putting forward, as (job_deadline, job_id).
    /// None when the server has no job at all, i.e. it is idle.
    pub current_job: Option<(SimTime, u64)>,
    /// The server's other pending jobs, ordered by (job_deadline, job_id) so the earliest
    /// deadline is served first and ties go to the older job.
    pub ready_queue: BinaryHeap<Reverse<(SimTime, u64)>>,
}

impl ServerDescriptor {
    /// A server starts with a full budget and a deadline of 0, so the first job that
    /// arrives always satisfies the recharge rule and starts a fresh (c, d) window.
    pub fn new(max_budget: SimTime, period: SimTime) -> Self {
        ServerDescriptor {
            max_budget,
            period,
            bandwidth: max_budget as f64 / period as f64,
            budget: max_budget,
            deadline: 0,
            current_job: None,
            ready_queue: BinaryHeap::new(),
        }
    }
}

/// Global ready queue holding one entry per waiting server, keyed by that server's
/// scheduling deadline. A server waits here under the id of the job it is currently
/// putting forward. Unlike the per-server heaps, entries here have to be removed by job
/// id while still queued (a job can lose its server's slot to a newer, more urgent one),
/// so this is a sorted set plus an id index rather than a plain BinaryHeap.
/// Insertion, removal and extraction are all O(log n).
pub struct JobsReadyQueue {
    entries: BTreeSet<(SimTime, u64)>,
    job_to_deadline: HashMap<u64, SimTime>,
}

impl JobsReadyQueue {
    pub fn new() -> Self {
        JobsReadyQueue {
            entries: BTreeSet::new(),
            job_to_deadline: HashMap::new(),
        }
    }

    /// Queue `job_id` under its server's deadline
    pub fn insert(&mut self, job_id: u64, server_deadline: SimTime) {
        self.entries.insert((server_deadline, job_id));
        self.job_to_deadline.insert(job_id, server_deadline);
    }

    /// Drop `job_id` from the queue, returns false if it was not queued
    pub fn remove(&mut self, job_id: u64) -> bool {
        let Some(server_deadline) = self.job_to_deadline.remove(&job_id) else {
            return false;
        };
        self.entries.remove(&(server_deadline, job_id));
        true
    }

    /// Extract the entry with the earliest server deadline, ties going to the older job
    pub fn pop_first(&mut self) -> Option<(SimTime, u64)> {
        let entry = self.entries.iter().next().copied()?;
        self.entries.remove(&entry);
        self.job_to_deadline.remove(&entry.1);
        Some(entry)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// This struct holds the global state of the simulation (the whole simulation are simply
/// handlers called on the basis of the extracted event that modify the state)
/// `event_id_to_task_descr` maps the `event_id` of a task to its descriptor
/// `event_id_to_job_descr` maps the `event_id` of a job to its descriptor
/// `event_id_to_server_descr` maps the `event_id` of a server to its descriptor
/// `jobs_on_cpu` is a vector holding the job_id running on the i^th CPU, None if idle
/// `jobs_ready_queue` holds the servers waiting for a CPU, ordered by server deadline
pub struct GlobalState {
    pub event_id_to_task_desc: HashMap<u64, TaskType>,
    pub event_id_to_job_descr: HashMap<u64, JobDescriptor>,
    pub event_id_to_server_descr: HashMap<u64, ServerDescriptor>,
    pub jobs_on_cpu: Vec<Option<u64>>,
    pub jobs_ready_queue: JobsReadyQueue,
}

impl GlobalState {
    pub fn new(num_cpus: usize) -> Self {
        GlobalState {
            event_id_to_task_desc: HashMap::new(),
            event_id_to_job_descr: HashMap::new(),
            event_id_to_server_descr: HashMap::new(),
            jobs_on_cpu: vec![None; num_cpus],
            jobs_ready_queue: JobsReadyQueue::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jobs_ready_queue_pops_earliest_deadline() {
        let mut queue = JobsReadyQueue::new();
        queue.insert(10, 50);
        queue.insert(11, 20);
        queue.insert(12, 30);

        assert_eq!(queue.pop_first(), Some((20, 11)));
        assert_eq!(queue.pop_first(), Some((30, 12)));
        assert_eq!(queue.pop_first(), Some((50, 10)));
        assert_eq!(queue.pop_first(), None);
    }

    #[test]
    fn test_jobs_ready_queue_ties_go_to_older_job() {
        let mut queue = JobsReadyQueue::new();
        queue.insert(11, 20);
        queue.insert(10, 20);

        assert_eq!(queue.pop_first(), Some((20, 10)));
        assert_eq!(queue.pop_first(), Some((20, 11)));
    }

    #[test]
    fn test_jobs_ready_queue_remove_by_job_id() {
        let mut queue = JobsReadyQueue::new();
        queue.insert(10, 50);
        queue.insert(11, 20);

        assert!(queue.remove(11));
        assert_eq!(queue.len(), 1);
        assert!(!queue.remove(11));
        assert_eq!(queue.pop_first(), Some((50, 10)));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_jobs_ready_queue_reinsert_with_new_deadline() {
        // A server that recharges while queued comes back under a later deadline
        let mut queue = JobsReadyQueue::new();
        queue.insert(10, 20);
        queue.insert(11, 30);

        let (_, job_id) = queue.pop_first().unwrap();
        assert_eq!(job_id, 10);
        queue.insert(job_id, 40);

        assert_eq!(queue.pop_first(), Some((30, 11)));
        assert_eq!(queue.pop_first(), Some((40, 10)));
    }
}
