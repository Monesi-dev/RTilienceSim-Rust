//! # Fixed-Priority Preemptive CPU Scheduler Simulator (Multi-Processor, Multi-Queue with Push/Pull)
//!
//! Unlike `mq` (static affinity), jobs here can migrate via push/pull load balancing:
//! - **Push (on arrival)**: if highest-priority job can preempt a running job, migrate it
//! - **Pull (on completion)**: scan all ready queues for the highest-priority waiting job
//!
//! Migrations are tracked and counted for performance analysis.

use crate::base::prelude::{SimTime, RandomVariable, Node, EventQueue};
use crate::util::get_next_id;
use super::state::{EventType, TaskType, JobDescriptor, GlobalState, EventPriorityPolicy};
use super::stats::Stats;
use std::cmp::Reverse;

pub struct CPUSimulatorMQLB<Q: EventQueue<EventType, EventPriorityPolicy>> {
    num_cpus: usize,
    state: GlobalState,
    queue: Q,
    stats: Stats,
}

impl<Q: EventQueue<EventType, EventPriorityPolicy>> CPUSimulatorMQLB<Q> {

    pub fn new(num_cpus: usize) -> Self {
        CPUSimulatorMQLB {
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

        self.stats.register_task(event_id, store_response_times);

        debug_println!("Added task type start_time={}, deadline={}, priority={}, cpu_affinity={}", start_time, relative_deadline, priority, cpu_affinity);
        self
    }

    fn schedule_job_on_cpu(&mut self, job_id: u64, cpu_idx: usize, current_time: SimTime) {
        self.state.jobs_on_cpu[cpu_idx] = Some(job_id);
        let job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
        job_desc.scheduled_on_cpu = Some((current_time, cpu_idx));
        let completion_time = current_time + job_desc.remaining_time;
        self.queue.insert(job_id, completion_time, EventType::JobCompletion).unwrap();
        self.stats.cpu_goes_busy(cpu_idx, current_time);

        debug_println!("Job-Id: {} scheduled on CPU {}. Completion-Time: {}", job_id, cpu_idx, completion_time);
    }

    fn preempt_job_on_cpu(&mut self, job_id: u64, cpu_idx: usize, current_time: SimTime) {
        let preempted_job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
        let (scheduled_time, _) = preempted_job_desc.scheduled_on_cpu.unwrap();
        let elapsed_time = current_time - scheduled_time;
        preempted_job_desc.remaining_time -= elapsed_time;
        preempted_job_desc.scheduled_on_cpu = None;
        self.queue.remove(scheduled_time + preempted_job_desc.remaining_time + elapsed_time, job_id);
        self.state.ready_queues[cpu_idx].push(Reverse((preempted_job_desc.priority, job_id)));
        self.stats.cpu_goes_idle(cpu_idx, current_time);

        debug_println!("Job-Id: {} preempted from CPU {}. Remaining-Time: {}", job_id, cpu_idx, preempted_job_desc.remaining_time);
    }

    fn find_idle_cpu(&self) -> Option<usize> {
        self.state.jobs_on_cpu.iter().position(|job| job.is_none())
    }

    /// Finds the CPU running the lowest-priority job. On a tie, favors `preferred_cpu`
    /// (the arriving task's affinity CPU) so a preemption doesn't cause a migration
    /// when it doesn't have to.
    fn find_lowest_priority_running_job(&self, preferred_cpu: usize) -> Option<(usize, u32)> {
        let preferred_job_id = self.state.jobs_on_cpu[preferred_cpu].unwrap();
        let preferred_priority = self.state.event_id_to_job_descr.get(&preferred_job_id).unwrap().priority;

        let mut lowest_cpu = preferred_cpu;
        let mut lowest_priority = preferred_priority;

        for (cpu_idx, job_id_opt) in self.state.jobs_on_cpu.iter().enumerate() {
            if cpu_idx == preferred_cpu {
                continue;
            }
            let job_id = job_id_opt.unwrap();
            let job_desc = self.state.event_id_to_job_descr.get(&job_id).unwrap();
            if job_desc.priority > lowest_priority {
                lowest_priority = job_desc.priority;
                lowest_cpu = cpu_idx;
            }
        }

        Some((lowest_cpu, lowest_priority))
    }

    // Find <priority, cpu> of cpu with lowest priority running job (with equal priority
    // give precedence to cpu of completed job)
    fn find_highest_priority_waiting_job(&self, current_cpu: usize) -> Option<(u64, usize)> {
        let mut best_job = None;
        let mut best_priority = u32::MAX;
        let mut best_cpu = current_cpu;

        // Check current CPU's ready queue first
        if let Some(&Reverse((priority, job_id))) = self.state.ready_queues[current_cpu].peek() {
            best_job = Some(job_id);
            best_priority = priority;
            best_cpu = current_cpu;
        }

        // Check other CPUs' ready queues
        for cpu_idx in 0..self.num_cpus {
            if cpu_idx == current_cpu {
                continue;
            }
            if let Some(&Reverse((priority, job_id))) = self.state.ready_queues[cpu_idx].peek() {
                if priority < best_priority {
                    best_job = Some(job_id);
                    best_priority = priority;
                    best_cpu = cpu_idx;
                }
            }
        }

        best_job.map(|job_id| (job_id, best_cpu))
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

    /// Handler for TaskTrigger event
    fn handle_task_trigger(&mut self, node: Node<EventType>) {
        let time = node.time;
        let event_id = node.event_id;
        debug_println!("TaskTrigger event extracted. Time: {}, Event-Id: {}", time, event_id);

        // Get Priority, Execution Time, Deadline and Affinity of new job from Task Descr
        let (priority, execution_time, deadline, cpu_affinity) = {
            let task_desc = self.state.event_id_to_task_desc.get_mut(&event_id).unwrap();
            debug_println!("TaskDescriptor Info: {:?}", task_desc);

            let delta = task_desc.interarrival_time_rng.sample();
            let new_time = time + delta;
            self.queue.insert(event_id, new_time, EventType::TaskTrigger).unwrap();

            let execution_time = task_desc.execution_time_rng.sample();
            let deadline = time + task_desc.relative_deadline;
            (task_desc.priority, execution_time, deadline, task_desc.cpu_affinity)
        };

        // Create Job Descriptor with its event_id mapping
        let job_id = get_next_id();
        let job_desc = JobDescriptor {
            scheduled_on_cpu: None,
            remaining_time: execution_time,
            priority,
            deadline,
            task_event_id: event_id,
        };
        self.state.event_id_to_job_descr.insert(job_id, job_desc);

        self.stats.register_job(job_id, time, event_id);
        debug_println!("Created JobDescriptor. Job-Id: {}, Execution-Time: {}, Deadline: {}, Priority: {}", job_id, execution_time, deadline, priority);

        // If a CPU is empty put job there (always give advantage to affine cpu), write
        // job_id in job_on_cpu vec, update job descriptor, insert JobCompletion and 
        // record migration
        if let Some(idle_cpu) = self.find_idle_cpu() {
            let target_cpu = if self.state.jobs_on_cpu[cpu_affinity].is_none() {
                cpu_affinity
            } else {
                idle_cpu
            };
            self.schedule_job_on_cpu(job_id, target_cpu, time);
            if target_cpu != cpu_affinity {
                self.stats.record_migration();
            }
            return;
        }

        // Otherwise check cpu with lowest priority job (with equal priority give 
        // precednece to affine cpu) and then if priority of new job is greater then 
        // preempt (remove JobCompletion, update preempted JobDescr and update job_on_cpu)
        // and put job in cpu (update job_on_cpu, job descriptor and insert JobCompletion)
        // and finally record migration
        if let Some((preempt_cpu, lowest_priority)) = self.find_lowest_priority_running_job(cpu_affinity) {
            if priority < lowest_priority {
                let preempted_job_id = self.state.jobs_on_cpu[preempt_cpu].unwrap();
                debug_println!("Job-Id: {} preempts Job-Id: {} on CPU {}", job_id, preempted_job_id, preempt_cpu);
                self.preempt_job_on_cpu(preempted_job_id, preempt_cpu, time);
                self.schedule_job_on_cpu(job_id, preempt_cpu, time);
                if preempt_cpu != cpu_affinity {
                    self.stats.record_migration();
                }
                return;
            }
        }

        // Otherwise add to affinity CPU's ready queue
        self.state.ready_queues[cpu_affinity].push(Reverse((priority, job_id)));
        debug_println!("Job-Id: {} added to ready queue of affinity CPU {}. Priority: {}", job_id, cpu_affinity, priority);
    }

    /// Handler for JobCompletion event
    fn handle_job_completion(&mut self, node: Node<EventType>) {
        let time = node.time;
        let job_id = node.event_id;
        debug_println!("JobCompletion event extracted. Time: {}, Job-Id: {}", time, job_id);

        // Get job deadline and cpu it was on from job descr
        let (job_deadline, current_cpu) = {
            let job_desc = self.state.event_id_to_job_descr.get(&job_id).unwrap();
            let (_, cpu_idx) = job_desc.scheduled_on_cpu.unwrap();
            (job_desc.deadline, cpu_idx)
        };

        assert_eq!(self.state.jobs_on_cpu[current_cpu], Some(job_id), "JobCompletion for job not on CPU");

        self.stats.record_job_completion(job_id, time, job_deadline);

        // Remove Job from jobs_on_cpu and remove job descriptorr
        self.state.jobs_on_cpu[current_cpu] = None;
        self.state.event_id_to_job_descr.remove(&job_id);
        debug_println!("Job-Id: {} completed and removed from CPU {}", job_id, current_cpu);

        self.stats.cpu_goes_idle(current_cpu, time);

        // Find the highest-priority waiting job across all CPUs (with equal priority
        // give precedence to ready queue on this cpu), pop from it and add job to cpu
        // (add JobCompletion, update job descriptor, update job_on_cpu) and record 
        // migration
        if let Some((next_job_id, job_cpu)) = self.find_highest_priority_waiting_job(current_cpu) {
            // Pop from the ready queue we identified
            self.state.ready_queues[job_cpu].pop();

            self.schedule_job_on_cpu(next_job_id, current_cpu, time);
            if current_cpu != job_cpu {
                self.stats.record_migration();
            }
        } else {
            debug_println!("No waiting job found. CPU {} is now idle.", current_cpu);
        }
    }

    fn handle_end_simulation(&mut self, node: Node<EventType>) {
        debug_println!("EndSimulation event extracted. Exiting");
        self.stats.finalize(node.time);
    }
}
