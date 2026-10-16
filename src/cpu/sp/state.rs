//! # State Structures for Single-Processor Scheduler
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

/// Tie-breaks events when they have the same time. `JobCompletion` has to happen before
/// `TaskTrigger` because a task arriving cannot see a cpu with a job with 0 time and
/// maybe preempt it (which doesn't make sense)
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
/// `scheduled_on_cpu` is the time at which it was scheduled on cpu if it the running
/// job otherwise it is None
/// `remaining_time` is the time from scheduled_on_cpu or, if None, from when it will be
/// scheduled remaining until completion
/// `priority` and `deadline` are the priority of job and its deadline
#[derive(Debug)]
pub struct JobDescriptor {
    pub scheduled_on_cpu: Option<SimTime>,
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
/// `job_in_cpu` holds the event_id of the job currently executed by the CPU, if CPU is
/// empty then it is None
/// `ready_queue` is a MinHeap keeping the Jobs waiting to be executed, it hold the pair
/// <priority, job_id>
pub struct GlobalState {
    pub event_id_to_task_desc: HashMap<u64, TaskType>,
    pub event_id_to_job_descr: HashMap<u64, JobDescriptor>,
    pub job_in_cpu: Option<u64>,
    pub ready_queue: BinaryHeap<Reverse<(u64, u64)>>,
}

impl GlobalState {
    pub fn new() -> Self {
        GlobalState {
            event_id_to_task_desc: HashMap::new(),
            event_id_to_job_descr: HashMap::new(),
            job_in_cpu: None,
            ready_queue: BinaryHeap::new(),
        }
    }
}
