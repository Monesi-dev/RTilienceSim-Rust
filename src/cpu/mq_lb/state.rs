//! # State Structures for Multi-Processor Multi-Queue Scheduler with Push/Pull Load Balancing
//!
//! Unlike `mq` (static affinity), jobs here can migrate across CPUs via push/pull mechanism.

use crate::base::prelude::{SimTime, RandomVariable, EventPriorities};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Reverse;

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

#[derive(Debug)]
pub struct TaskType {
    pub relative_deadline: SimTime,
    pub priority: u32,
    pub execution_time_rng: Box<dyn RandomVariable>,
    pub interarrival_time_rng: Box<dyn RandomVariable>,
    pub cpu_affinity: usize,
}

/// JobDescriptor now tracks `scheduled_on_cpu` as Option<(time, cpu_idx)>
/// to support job migration across CPUs.
#[derive(Debug)]
pub struct JobDescriptor {
    pub scheduled_on_cpu: Option<(SimTime, usize)>,
    pub remaining_time: SimTime,
    pub priority: u32,
    pub deadline: SimTime,
    pub task_event_id: u64,
}

pub struct GlobalState {
    pub event_id_to_task_desc: HashMap<u64, TaskType>,
    pub event_id_to_job_descr: HashMap<u64, JobDescriptor>,
    pub jobs_on_cpu: Vec<Option<u64>>,
    pub ready_queues: Vec<BinaryHeap<Reverse<(u32, u64)>>>,
    pub cpu_utilization: Vec<f64>,
}

impl GlobalState {
    pub fn new(num_cpus: usize) -> Self {
        GlobalState {
            event_id_to_task_desc: HashMap::new(),
            event_id_to_job_descr: HashMap::new(),
            jobs_on_cpu: vec![None; num_cpus],
            ready_queues: (0..num_cpus).map(|_| BinaryHeap::new()).collect(),
            cpu_utilization: vec![0.0; num_cpus],
        }
    }
}
