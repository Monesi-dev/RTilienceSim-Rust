//! # Fixed-Priority Preemptive CPU Scheduler Simulator (Multi-Processor)
//!
//! This module simulates fixed-priority scheduling on M processors with a single global queue.
//! The simulator uses an event-driven approach where:
//! - **Events** are extracted from a priority queue ordered by simulation time
//! - **Handlers** are called for each event type, modifying priority queue and global state
//! - **Processors** have a global ready queue; the M highest-priority jobs run on M processors
//!

use crate::base::prelude::{SimTime, RandomVariable, Node, EventQueue};
use crate::util::get_next_id;
use super::state::{EventType, TaskType, JobDescriptor, GlobalState, EventPriorityPolicy};
use super::stats::Stats;
use std::cmp::Reverse;

/// Generic over Q: EventQueue<EventType, EventPriorityPolicy> for static dispatch to any
/// queue implementation.
pub struct CPUSimulatorMP<Q: EventQueue<EventType, EventPriorityPolicy>> {
    state: GlobalState,
    queue: Q,
    stats: Stats,
}

impl<Q: EventQueue<EventType, EventPriorityPolicy>> CPUSimulatorMP<Q> {

    pub fn new(num_cpus: usize) -> Self {
        CPUSimulatorMP {
            state: GlobalState::new(num_cpus),
            queue: Q::new(),
            stats: Stats::new(num_cpus),
        }
    }

    pub fn with_end_time(mut self, end_time: SimTime) -> Self {
        self.queue.insert(get_next_id(), end_time, EventType::EndSimulation).unwrap();
        self
    }

    // Builder method to set up simulation
    pub fn add_task_type(
        mut self,
        start_time: SimTime,
        relative_deadline: SimTime,
        priority: u32,
        execution_time_rng: Box<dyn RandomVariable>,
        interarrival_time_rng: Box<dyn RandomVariable>,
        store_response_times: bool,
    ) -> Self {

        // Create Task Descriptor, Insert TaskTrigger event in priority queue and 
        // update event_id->task_descr mapping
        let task = TaskType {
            relative_deadline,
            priority,
            execution_time_rng,
            interarrival_time_rng,
        };
        let event_id: u64 = get_next_id();
        self.queue.insert(event_id, start_time, EventType::TaskTrigger).unwrap();
        self.state.event_id_to_task_desc.insert(event_id, task);

        // Sets up task in stats object, pass the flag to tell whether to store times
        self.stats.register_task(event_id, store_response_times);

        debug_println!("Added task type start_time={}, deadline={}, priority={}", start_time, relative_deadline, priority);
        self
    }

    /// Schedule a job on a specific CPU
    fn schedule_job_on_cpu(&mut self, job_id: u64, cpu_idx: usize, current_time: SimTime) {

        // Writes job_id on vector mapping i^th cpu to id of job running on it
        self.state.jobs_on_cpu[cpu_idx] = Some(job_id);

        // Access job descriptor and writes scheduled_on_cpu (so time + id of cpu) and 
        // also insert JobCompletoin event in priority queue
        let job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
        job_desc.scheduled_on_cpu = Some((current_time, cpu_idx));
        let completion_time = current_time + job_desc.remaining_time;
        self.queue.insert(job_id, completion_time, EventType::JobCompletion).unwrap();

        // Tell stats object that cpu has been occpuied by a job
        self.stats.cpu_goes_busy(cpu_idx, current_time);

        debug_println!("Job-Id: {} scheduled on CPU {}. Completion-Time: {}", job_id, cpu_idx, completion_time);
    }

    /// Preempt a job on a specific CPU
    fn preempt_job_on_cpu(&mut self, job_id: u64, cpu_idx: usize, current_time: SimTime) {
        
        // Get job struct, update its remaining time and set scheduled on cpu to None
        let preempted_job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
        let (scheduled_time, _) = preempted_job_desc.scheduled_on_cpu.unwrap();
        let elapsed_time = current_time - scheduled_time;
        preempted_job_desc.remaining_time -= elapsed_time;
        preempted_job_desc.scheduled_on_cpu = None;

        // Remove the JobCompletion Event and push job to ready queue
        self.queue.remove(scheduled_time + preempted_job_desc.remaining_time + elapsed_time, job_id);
        self.state.ready_queue.push(Reverse((preempted_job_desc.priority, job_id)));

        // Tells the sttas object that cpu has not a job on it
        self.stats.cpu_goes_idle(cpu_idx, current_time);

        debug_println!("Job-Id: {} preempted from CPU {}. Remaining-Time: {}", job_id, cpu_idx, preempted_job_desc.remaining_time);
    }

    /// Find the CPU with the lowest-priority job currently running
    fn find_cpu_with_lowest_priority_job(&self) -> Option<(usize, u32)> {
        let mut lowest_cpu = None;
        let mut lowest_priority = u32::MIN;

        // Iterate over job on cpu, if there is a job then access job descr and if it 
        // has a lower priority than current lowest then set lowest to this job
        for (cpu_idx, job_id_opt) in self.state.jobs_on_cpu.iter().enumerate() {
            if let Some(job_id) = job_id_opt {
                let job_desc = self.state.event_id_to_job_descr.get(job_id).unwrap();
                if job_desc.priority > lowest_priority {
                    lowest_priority = job_desc.priority;
                    lowest_cpu = Some((cpu_idx, lowest_priority));
                }
            }
        }

        lowest_cpu
    }

    /// Find an idle CPU, returns index if found
    fn find_idle_cpu(&self) -> Option<usize> {
        self.state.jobs_on_cpu.iter().position(|job| job.is_none())
    }

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

    pub fn print_stats(&self) {
        self.stats.print_summary();
    }

    pub fn dump_response_times(&self, filename: &str) -> std::io::Result<()> {
        self.stats.dump_response_times_to_file(filename)
    }

    fn handle_task_trigger(&mut self, node: Node<EventType>) {
        let time = node.time;
        let event_id = node.event_id;
        debug_println!("TaskTrigger event extracted. Time: {}, Event-Id: {}", time, event_id);

        // Extract data from task_desc
        let (priority, execution_time, deadline) = {
            let task_desc = self.state.event_id_to_task_desc.get_mut(&event_id).unwrap();
            debug_println!("TaskDescriptor Info: {:?}", task_desc);

            // Schedule next TaskTrigger event
            let delta = task_desc.interarrival_time_rng.sample();
            let new_time = time + delta;
            self.queue.insert(event_id, new_time, EventType::TaskTrigger).unwrap();

            let execution_time = task_desc.execution_time_rng.sample();
            let deadline = time + task_desc.relative_deadline;
            (task_desc.priority, execution_time, deadline)
        };

        // Create JobDescriptor for the new job
        let job_id = get_next_id();
        let job_desc = JobDescriptor {
            scheduled_on_cpu: None,
            remaining_time: execution_time,
            priority,
            deadline,
        };
        self.state.event_id_to_job_descr.insert(job_id, job_desc);

        self.stats.register_job(job_id, time, event_id);
        debug_println!("Created JobDescriptor. Job-Id: {}, Execution-Time: {}, Deadline: {}, Priority: {}", job_id, execution_time, deadline, priority);

        // If there is an idle cpu schedule on it
        if let Some(idle_cpu) = self.find_idle_cpu() {
            self.schedule_job_on_cpu(job_id, idle_cpu, time);
            return;
        }
        // If no idle CPU, get CPU with lowest priority and if priority of this task
        // is higher then preempt and schedule otherwse just add to ready_queue
        let (preempt_cpu, lowest_priority) = self.find_cpu_with_lowest_priority_job().unwrap();
        if priority < lowest_priority {
            let preempted_job_id = self.state.jobs_on_cpu[preempt_cpu].unwrap();
            debug_println!("Job-Id: {} preempts Job-Id: {} on CPU {}", job_id, preempted_job_id, preempt_cpu);
            self.preempt_job_on_cpu(preempted_job_id, preempt_cpu, time);
            self.schedule_job_on_cpu(job_id, preempt_cpu, time);
        } else {
            self.state.ready_queue.push(Reverse((priority, job_id)));
            debug_println!("Job-Id: {} added to ready queue. Priority: {}", job_id, priority);
        }
    }

    fn handle_job_completion(&mut self, node: Node<EventType>) {
        let time = node.time;
        let job_id = node.event_id;
        debug_println!("JobCompletion event extracted. Time: {}, Job-Id: {}", time, job_id);

        // Find which CPU this job was on
        let cpu_idx = {
            let job_desc = self.state.event_id_to_job_descr.get(&job_id).unwrap();
            let (_, cpu_idx) = job_desc.scheduled_on_cpu.unwrap();
            cpu_idx
        };

        // Verify job is on this CPU
        assert_eq!(self.state.jobs_on_cpu[cpu_idx], Some(job_id), "JobCompletion for job not on CPU");

        // Record job completion in statistics
        let job_deadline = self.state.event_id_to_job_descr.get(&job_id).unwrap().deadline;
        self.stats.record_job_completion(job_id, time, job_deadline);

        // Remove completed job from CPU
        self.state.jobs_on_cpu[cpu_idx] = None;
        self.state.event_id_to_job_descr.remove(&job_id);
        debug_println!("Job-Id: {} completed and removed from CPU {}", job_id, cpu_idx);

        // Stop counting this CPU's time
        self.stats.cpu_goes_idle(cpu_idx, time);

        // Schedule next job from ready queue if available
        if let Some(Reverse((_, next_job_id))) = self.state.ready_queue.pop() {
            self.schedule_job_on_cpu(next_job_id, cpu_idx, time);
        } else {
            debug_println!("Ready queue is empty. CPU {} is now idle.", cpu_idx);
        }
    }

    fn handle_end_simulation(&mut self, node: Node<EventType>) {
        debug_println!("EndSimulation event extracted. Exiting");
        self.stats.finalize(node.time);
    }
}
