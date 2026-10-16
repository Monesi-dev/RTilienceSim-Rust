//! # Fixed-Priority Preemptive CPU Scheduler Simulator (Multi-Processor, Multi-Queue)
//!
//! This module simulates fixed-priority scheduling on M processors where each processor
//! has its own local ready queue. Unlike `mp` (one global ready queue shared by all
//! CPUs), here every task is pinned to a single CPU (its affinity) at registration
//! time, chosen as the CPU with the lowest accumulated estimated utilization so far.
//! A job can therefore only ever run, preempt, or wait on the CPU its task is pinned to.
//!
//! The simulator still drains a single, globally time-ordered event queue, since event
//! ordering is a property of the whole system timeline, not of any one CPU.

use crate::base::prelude::{SimTime, RandomVariable, Node, EventQueue};
use crate::util::get_next_id;
use super::state::{EventType, TaskType, JobDescriptor, GlobalState, EventPriorityPolicy};
use super::stats::Stats;
use std::cmp::Reverse;

/// Generic over Q: EventQueue<EventType, EventPriorityPolicy> for static dispatch to any
/// queue implementation.
pub struct CPUSimulatorMQ<Q: EventQueue<EventType, EventPriorityPolicy>> {
    num_cpus: usize,
    state: GlobalState,
    queue: Q,
    stats: Stats,
}

impl<Q: EventQueue<EventType, EventPriorityPolicy>> CPUSimulatorMQ<Q> {

    pub fn new(num_cpus: usize) -> Self {
        CPUSimulatorMQ {
            num_cpus,
            state: GlobalState::new(num_cpus),
            queue: Q::new(),
            stats: Stats::new(num_cpus),
        }
    }

    pub fn with_end_time(mut self, end_time: SimTime) -> Self {
        self.queue.insert(get_next_id(), end_time, EventType::EndSimulation).unwrap();
        self
    }

    /// Registers a task type. To pick its CPU affinity, it samples both RVs once to get
    /// a point estimate of the task's utilization (execution_time / interarrival_time)
    /// and assigns the task to whichever CPU currently has the lowest accumulated
    /// estimated utilization, then updates that accumulator.
    pub fn add_task_type(
        mut self,
        start_time: SimTime,
        relative_deadline: SimTime,
        priority: u32,
        mut execution_time_rng: Box<dyn RandomVariable>,
        mut interarrival_time_rng: Box<dyn RandomVariable>,
        store_response_times: bool,
    ) -> Self {

        // Estimate utilization by sampling both RVs once
        let execution_sample = execution_time_rng.sample();
        let interarrival_sample = interarrival_time_rng.sample();
        let utilization = execution_sample as f64 / interarrival_sample as f64;

        // Assign to the CPU with the lowest accumulated utilization so far
        let mut min_cpu_idx = 0;
        let mut min_cpu_util = f64::MAX;
        for (cpu_idx, &cpu_util) in self.state.cpu_utilization.iter().enumerate() {
            if cpu_util < min_cpu_util {
                min_cpu_util = cpu_util;
                min_cpu_idx = cpu_idx;
            }

        }
        let cpu_affinity = min_cpu_idx;

        self.state.cpu_utilization[cpu_affinity] += utilization;

        // Insert TaskType event in priority queue and add Task Desc to global state
        let task = TaskType {
            relative_deadline,
            priority,
            execution_time_rng,
            interarrival_time_rng,
            cpu_affinity,
        };
        let event_id: u64 = get_next_id();
        self.queue.insert(event_id, start_time, EventType::TaskTrigger).unwrap();
        self.state.event_id_to_task_desc.insert(event_id, task);

        // Updating statistics
        self.stats.register_task(event_id, store_response_times);

        debug_println!("Added task type start_time={}, deadline={}, priority={}, cpu_affinity={}", start_time, relative_deadline, priority, cpu_affinity);
        self
    }

    /// Private helper method used to "schedule" job on a specific cpu.
    /// It takes job_id, cpu_idx and time, sets job_id as scheduled on that cpu, updates
    /// job descriptor and adds JobCompletion event
    fn schedule_job_on_cpu(&mut self, job_id: u64, cpu_idx: usize, current_time: SimTime) {

        self.state.jobs_on_cpu[cpu_idx] = Some(job_id);
        let job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
        job_desc.scheduled_on_cpu = Some(current_time);
        let completion_time = current_time + job_desc.remaining_time;
        self.queue.insert(job_id, completion_time, EventType::JobCompletion).unwrap();
        self.stats.cpu_goes_busy(cpu_idx, current_time);

        debug_println!("Job-Id: {} scheduled on CPU {}. Completion-Time: {}", job_id, cpu_idx, completion_time);
    }

    /// Private helper method used to preempt a running job on a specific cpu.
    /// It takes job_id, cpu_idx and current_time, updates the job descriptor,
    /// removes the JobCompletion event, and adds the job to that cpu's ready queue.
    fn preempt_job_on_cpu(&mut self, job_id: u64, cpu_idx: usize, current_time: SimTime) {

        let preempted_job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
        let scheduled_time = preempted_job_desc.scheduled_on_cpu.unwrap();
        let elapsed_time = current_time - scheduled_time;
        preempted_job_desc.remaining_time -= elapsed_time;
        preempted_job_desc.scheduled_on_cpu = None;
        self.queue.remove(scheduled_time + preempted_job_desc.remaining_time + elapsed_time, job_id);
        self.state.ready_queues[cpu_idx].push(Reverse((preempted_job_desc.priority, job_id)));
        self.stats.cpu_goes_idle(cpu_idx, current_time);

        debug_println!("Job-Id: {} preempted from CPU {}. Remaining-Time: {}", job_id, cpu_idx, preempted_job_desc.remaining_time);
    }

    /// Public method to run simulation.
    /// It is a loop of (ExtractEvent -> CallHandler) until EndSimulation event
    pub fn simulate(&mut self) {

        debug_println!("Simulate method invoked");

        loop {

            debug_println!("Extracting next event ...");

            let node = self.queue.pop_first().unwrap();

            match node.event_type {
                EventType::TaskTrigger => self.handle_task_trigger(node),
                EventType::JobCompletion => self.handle_job_completion(node),
                EventType::EndSimulation => {
                    self.handle_end_simulation(node);
                    break;
                }
            }
        }
    }

    /// Print simulation statistics summary
    pub fn print_stats(&self) {
        self.stats.print_summary();
    }

    /// Dump response times to a CSV file
    pub fn dump_response_times(&self, filename: &str) -> std::io::Result<()> {
        self.stats.dump_response_times_to_file(filename)
    }

    /// Private handler for TaskTrigger events.
    /// Creates a new job from the task and schedules it (or adds it to its cpu's ready
    /// queue). Since the task is pinned to a fixed cpu, only that cpu is ever considered.
    fn handle_task_trigger(&mut self, node: Node<EventType>) {
        let time = node.time;
        let event_id = node.event_id;
        debug_println!("TaskTrigger event extracted. Time: {}, Event-Id: {}", time, event_id);

        // Extract data from task_desc before calling other methods to avoid borrow issues
        let (priority, execution_time, deadline, cpu_idx) = {
            let task_desc = self.state.event_id_to_task_desc.get_mut(&event_id).unwrap();
            debug_println!("TaskDescriptor Info: {:?}", task_desc);

            // Schedule next TaskTrigger event
            let delta = task_desc.interarrival_time_rng.sample();
            let new_time = time + delta;
            self.queue.insert(event_id, new_time, EventType::TaskTrigger).unwrap();
            debug_println!("Inserted TaskTrigger back in the queue. NewTime: {}, Event-Id: {}, Delta: {}", new_time, event_id, delta);

            // Extract what we need from task_desc
            let execution_time = task_desc.execution_time_rng.sample();
            let deadline = time + task_desc.relative_deadline;
            (task_desc.priority, execution_time, deadline, task_desc.cpu_affinity)
        }; // task_desc reference dropped here

        // Create JobDescriptor for the new job
        let job_id = get_next_id();
        let job_desc = JobDescriptor {
            scheduled_on_cpu: None,
            remaining_time: execution_time,
            priority,
            deadline,
            task_event_id: event_id,
        };
        self.state.event_id_to_job_descr.insert(job_id, job_desc);

        // Updating Statistics
        self.stats.register_job(job_id, time, event_id);
        debug_println!("Created JobDescriptor. Job-Id: {}, Execution-Time: {}, Deadline: {}, Priority: {}, Cpu: {}", job_id, execution_time, deadline, priority, cpu_idx);

        // If this task's cpu is idle schedule on it
        if self.state.jobs_on_cpu[cpu_idx].is_none() {
            self.schedule_job_on_cpu(job_id, cpu_idx, time);
            return;
        }
        // Otherwise if the running job on that cpu has lower priority preempt and
        // schedule on it. If it has higher priority just add to that cpu's ready queue
        let running_job_id = self.state.jobs_on_cpu[cpu_idx].unwrap();
        let running_job_priority = self.state.event_id_to_job_descr.get(&running_job_id).unwrap().priority;

        if priority < running_job_priority {
            debug_println!("Job-Id: {} preempts Job-Id: {} on CPU {}", job_id, running_job_id, cpu_idx);
            self.preempt_job_on_cpu(running_job_id, cpu_idx, time);
            self.schedule_job_on_cpu(job_id, cpu_idx, time);
        } else {
            self.state.ready_queues[cpu_idx].push(Reverse((priority, job_id)));
            debug_println!("Job-Id: {} added to ready queue of CPU {}. Priority: {}", job_id, cpu_idx, priority);
        }
    }

    /// Private handler for JobCompletion events.
    /// Removes the completed job and schedules the next one from that cpu's ready queue.
    fn handle_job_completion(&mut self, node: Node<EventType>) {
        let time = node.time;
        let job_id = node.event_id;
        debug_println!("JobCompletion event extracted. Time: {}, Job-Id: {}", time, job_id);

        // Find which cpu this job's task is pinned to
        let (task_event_id, job_deadline) = {
            let job_desc = self.state.event_id_to_job_descr.get(&job_id).unwrap();
            (job_desc.task_event_id, job_desc.deadline)
        };
        let cpu_idx = self.state.event_id_to_task_desc.get(&task_event_id).unwrap().cpu_affinity;

        // Verify job is the one running on that cpu
        assert_eq!(self.state.jobs_on_cpu[cpu_idx], Some(job_id), "JobCompletion for job not on CPU");

        // Record job completion in statistics before removing
        self.stats.record_job_completion(job_id, time, job_deadline);

        // Remove completed job
        self.state.jobs_on_cpu[cpu_idx] = None;
        self.state.event_id_to_job_descr.remove(&job_id);
        debug_println!("Job-Id: {} completed and removed from CPU {}", job_id, cpu_idx);

        // Stop counting the current CPU time
        self.stats.cpu_goes_idle(cpu_idx, time);

        // Schedule next job from this cpu's ready queue if available
        if let Some(Reverse((_, next_job_id))) = self.state.ready_queues[cpu_idx].pop() {
            self.schedule_job_on_cpu(next_job_id, cpu_idx, time);
        } else {
            debug_println!("Ready queue of CPU {} is empty. CPU is now idle.", cpu_idx);
        }
    }

    /// Private handler for EndSimulation events.
    /// Any job still running or sitting in a ready queue never reached
    /// `handle_job_completion`, so it is accounted for here: if its deadline has
    /// already passed, it's recorded as an unmet-and-uncompleted job.
    fn handle_end_simulation(&mut self, node: Node<EventType>) {
        debug_println!("EndSimulation event extracted. Exiting");

        for (job_id, job_desc) in &self.state.event_id_to_job_descr {
            self.stats.record_unfinished_job(*job_id, job_desc.deadline, node.time);
        }

        self.stats.finalize(node.time);
    }
}
