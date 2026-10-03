//! # State Structures for Multi-Processor Scheduler
//!
//! Contains the data structures that represent the state of the simulation.

use crate::base::prelude::{SimTime, RandomVariable};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Reverse;

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    TaskTrigger,
    JobCompletion,
    EndSimulation,
}

#[derive(Debug)]
pub struct TaskType {
    pub relative_deadline: SimTime,
    pub priority: u32,
    pub execution_time_rng: Box<dyn RandomVariable>,
    pub interarrival_time_rng: Box<dyn RandomVariable>,
}

/// Job descriptor for multi-processor system.
/// `scheduled_on_cpu` holds both the scheduling time and the CPU index (0 to M-1)
#[derive(Debug)]
pub struct JobDescriptor {
    pub scheduled_on_cpu: Option<(SimTime, usize)>,
    pub remaining_time: SimTime,
    pub priority: u32,
    pub deadline: SimTime,
}

pub struct GlobalState {
    pub event_id_to_task_desc: HashMap<u64, TaskType>,
    pub event_id_to_job_descr: HashMap<u64, JobDescriptor>,
    pub jobs_on_cpu: Vec<Option<u64>>,  // One slot per CPU, holds job_id or None if idle
    pub ready_queue: BinaryHeap<Reverse<(u32, u64)>>,
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
