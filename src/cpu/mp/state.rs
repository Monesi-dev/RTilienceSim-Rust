//! # State Structures for Multi-Processor Scheduler
//!
//! Contains the data structures that represent the state of the simulation.

use crate::base::prelude::{SimTime, RandomVariable, EventPriorities};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Reverse;

/// Enum with Event types used in this simulation
/// `TaskTrigger` means that a task is about to spawn a job
/// `JobCompletion` means that job being executed has ended
/// `EndSimulation` means that simulation has to stop
#[derive(Debug, Clone, Copy)]
pub enum EventType {
    TaskTrigger,
    JobCompletion,
    EndSimulation,
}

/// Tie-breaks events sharing the same simulation time. `JobCompletion` is given
/// priority over `TaskTrigger` so that a job finishing and a task arriving at the same
/// instant always see the CPU as already freed, regardless of insertion order.
#[derive(Debug, Clone, Copy)]
pub struct EventPriorityPolicy;

impl EventPriorities<EventType> for EventPriorityPolicy {
    fn priority(event_type: &EventType) -> u32 {
        match event_type {
            EventType::JobCompletion => 0,
            EventType::TaskTrigger => 1,
            EventType::EndSimulation => 2,
        }
    }
}

/// This is a Task Descriptor struct, it is used to store, in the global state, everything
/// that should be remembered about a task.
/// `relative_deadline` and `priority` are used to get deadline and priority of spawned job.
/// `execution_time_rng` is used to sample execution time of spawned job.
/// `interarrival_time_rng` is used to sample time after which a new job will be spawned.
#[derive(Debug)]
pub struct TaskType {
    pub relative_deadline: SimTime,
    pub priority: u64,
    pub execution_time_rng: Box<dyn RandomVariable>,
    pub interarrival_time_rng: Box<dyn RandomVariable>,
}

/// This is a Job Descriptor struct, it is used to store, in the global state, everything
/// that should be known about a job.
/// `scheduled_on_cpu` is the time  at which it was scheduled on cpu and idx of the cpu 
/// if it is a running job otherwise it is None
/// `remaining_time` is the time from scheduled_on_cpu or, if None, from when it will be
/// scheduled remaining until completion
/// `priority` and `deadline` are the priority of job and its deadline
#[derive(Debug)]
pub struct JobDescriptor {
    pub scheduled_on_cpu: Option<(SimTime, usize)>,
    pub remaining_time: SimTime,
    pub priority: u64,
    pub deadline: SimTime,
}

/// This struct holds the global state of the simulation (the whole simulation are simply
/// handlers called on the basis of the extracted event that modify the state)
/// `event_id_to_task_descr` is used to map the `event_id` of the message linked to a
/// specific task to its descriptor
/// `event_id_to_job_descr` is used to map the `event_id` of the messages linked to a
/// specific job to its descriptor
/// `job_in_cpu` is a vector holding the event_id of the job currently executed by the 
/// i^th CPU, if i^th CPU is idle then it is None
/// `ready_queue` is a MinHeap keeping the Jobs waiting to be executed, it hold the pair
/// <priority, job_id>
pub struct GlobalState {
    pub event_id_to_task_desc: HashMap<u64, TaskType>,
    pub event_id_to_job_descr: HashMap<u64, JobDescriptor>,
    pub jobs_on_cpu: Vec<Option<u64>>,  // One slot per CPU, holds job_id or None if idle
    pub ready_queue: BinaryHeap<Reverse<(u64, u64)>>,
}

impl GlobalState {
    pub fn new(num_cpus: usize) -> Self {
        GlobalState {
            event_id_to_task_desc: HashMap::new(),
            event_id_to_job_descr: HashMap::new(),
            jobs_on_cpu: vec![None; num_cpus],
            ready_queue: BinaryHeap::new(),
        }
    }
}
