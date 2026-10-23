//! # CBS Preemptive CPU Scheduler Simulator (Multi-Processor, Multi-Queue)
//!
//! This module simulates Constant Bandwidth Server scheduling on M processors where each
//! server is pinned to a cpu and each cpu has its own queue of servers. In `mp_cbs` every
//! server competes in one global queue and can end up on any cpu, here a server is
//! assigned a cpu once at registration time and never leaves it, so the system is really
//! M independent uniprocessor CBS schedulers. The scheduling is still on 2 levels:
//!
//! - **Between servers of a cpu**: servers pinned to a cpu compete for it using the
//!   server deadline as priority, basically EDF between the servers of that cpu. A server
//!   deadline can change when its budget is emptied and refilled.
//! - **Inside a server**: When a server is scheduled only one of its jobs is executed,
//!   it is chosen using an internal priority, which is the job's deadline
//!
//! The CBS rules themselves are:
//! - On being dispatched after not running, if `c >= (d - t) * U` the old budget would
//!   give the server more than its share, so it restarts its window: `c = Q, d = t + T`.
//! - On running out of budget, the server is replenished and postponed: `c += Q, d += T`,
//!   which pushes it back in its cpu's ordering and lets the other servers in.
//!
//! Servers are spread over the cpus at registration time by always picking the cpu with
//! the lowest accumulated bandwidth. Unlike `mq`, which has to sample the random
//! variables once to guess a task's utilization, here the reservation `Q/T` is declared,
//! so the assignment is exact and deterministic.

use crate::base::prelude::{SimTime, Node, EventQueue};
use crate::util::get_next_id;
use super::state::{
    EventType, EventPriorityPolicy, GlobalState, JobDescriptor, ServerDescriptor, TaskSpec, TaskType,
};
use super::stats::Stats;
use std::cmp::Reverse;

/// Generic over Q: EventQueue<EventType, EventPriorityPolicy> for static dispatch to any
/// queue implementation. There is no Scheduler generic here: CBS fixes both levels of
/// priority itself (job deadline inside a server, server deadline between servers).
pub struct CPUSimulatorMQCBS<Q: EventQueue<EventType, EventPriorityPolicy>> {
    state: GlobalState,
    queue: Q,
    stats: Stats,
}

impl<Q: EventQueue<EventType, EventPriorityPolicy>> CPUSimulatorMQCBS<Q> {

    pub fn new(num_cpus: usize) -> Self {
        CPUSimulatorMQCBS {
            state: GlobalState::new(num_cpus),
            queue: Q::new(),
            stats: Stats::new(num_cpus),
        }
    }

    /// Builder method inserting event to stop simulation at a given time horizon
    pub fn with_end_time(mut self, end_time: SimTime) -> Self {
        self.queue.insert(get_next_id(), end_time, EventType::EndSimulation).unwrap();
        self
    }

    /// Builder method that creates one server with budget `max_budget` (Q) and period
    /// `period` (T), pins it to the least loaded cpu, and registers every task in `tasks`
    /// as served by it. All of these tasks share its budget and its cpu.
    pub fn add_server(mut self, max_budget: SimTime, period: SimTime, tasks: Vec<TaskSpec>) -> Self {

        // Assign the server to the cpu with the lowest accumulated bandwidth so far
        let mut min_cpu_idx = 0;
        let mut min_cpu_utilization = f64::MAX;
        for (cpu_idx, &cpu_utilization) in self.state.utilization_on_cpu.iter().enumerate() {
            if cpu_utilization < min_cpu_utilization {
                min_cpu_utilization = cpu_utilization;
                min_cpu_idx = cpu_idx;
            }
        }

        // Create the server descriptor, charge its bandwidth to the cpu it was given and
        // add its event_id -> server_descr mapping
        let server_id = get_next_id();
        let server_desc = ServerDescriptor::new(max_budget, period, min_cpu_idx);
        self.state.utilization_on_cpu[min_cpu_idx] += server_desc.bandwidth;
        self.state.event_id_to_server_descr.insert(server_id, server_desc);
        debug_println!("Added server Server-Id: {}, Q={}, T={}, affine CPU {}",
                       server_id, max_budget, period, min_cpu_idx);

        // Iterate over tasks, create the task descriptor (linking it to the server) and
        // add the TaskTrigger event for each of them
        for task in tasks {
            let task_id = get_next_id();
            self.queue.insert(task_id, task.start_time, EventType::TaskTrigger).unwrap();
            self.state.event_id_to_task_desc.insert(task_id, TaskType {
                relative_deadline: task.relative_deadline,
                execution_time_rng: task.execution_time_rng,
                interarrival_time_rng: task.interarrival_time_rng,
                server_id,
            });
            self.stats.register_task(task_id, task.store_response_times);

            debug_println!("Added task Task-Id: {} to Server-Id: {}, start_time={}, deadline={}",
                           task_id, server_id, task.start_time, task.relative_deadline);
        }

        self
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
                EventType::BudgetExhausted => self.handle_budget_exhausted(node),
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

    /// Read-only access to the collected statistics
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Private helper walking job -> task -> server to get the id of the server that
    /// serves a given job
    fn server_id_of_job(&self, job_id: u64) -> u64 {
        let task_id = self.state.event_id_to_job_descr.get(&job_id).unwrap().task_id;
        self.state.event_id_to_task_desc.get(&task_id).unwrap().server_id
    }

    /// Private helper giving the cpu a server is pinned to
    fn affine_cpu_of_server(&self, server_id: u64) -> usize {
        self.state.event_id_to_server_descr.get(&server_id).unwrap().affine_cpu
    }

    /// Private helper that stops a job from executing: it charges the time it ran for to
    /// the job's remaining time and to its server's budget, and deletes its JobCompletion
    /// event. It leaves BudgetExhausted and jobs_on_cpu alone, so the caller decides
    /// whether the server keeps the cpu or gives it up.
    /// Returns the index of the cpu the job was running on.
    fn stop_job_execution(&mut self, job_id: u64, current_time: SimTime) -> usize {

        let server_id = self.server_id_of_job(job_id);
        let cpu_idx = self.affine_cpu_of_server(server_id);

        // Charge the run time to job and compute the completion time so JobCompletion
        // event can be removed
        let (scheduled_time, completion_time) = {
            let job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
            let scheduled_time = job_desc.scheduled_on_cpu.unwrap();
            let completion_time = scheduled_time + job_desc.remaining_time;
            job_desc.remaining_time -= current_time - scheduled_time;
            job_desc.scheduled_on_cpu = None;
            (scheduled_time, completion_time)
        };
        self.queue.remove(completion_time, job_id);

        // Charge the same time to the server's budget
        let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();
        server_desc.budget -= current_time - scheduled_time;

        cpu_idx
    }

    /// Private helper that takes a running job off its cpu without finishing it: on top
    /// of charging the run time it also deletes the BudgetExhausted event, since the
    /// server stops consuming budget, and frees the cpu.
    /// Returns the index of the cpu that was freed.
    fn suspend_running_job(&mut self, job_id: u64, current_time: SimTime) -> usize {

        let server_id = self.server_id_of_job(job_id);

        // The BudgetExhausted was armed for the instant the budget would have run out,
        // so it has to be computed before stop_job_execution charges the run to it
        let exhausted_time = {
            let server_desc = self.state.event_id_to_server_descr.get(&server_id).unwrap();
            let job_desc = self.state.event_id_to_job_descr.get(&job_id).unwrap();
            job_desc.scheduled_on_cpu.unwrap() + server_desc.budget
        };

        let cpu_idx = self.stop_job_execution(job_id, current_time);
        self.queue.remove(exhausted_time, server_id);

        // Sets jobs_on_cpu to None
        self.state.jobs_on_cpu[cpu_idx] = None;
        self.stats.cpu_goes_idle(cpu_idx, current_time);

        debug_println!("Job-Id: {} of Server-Id: {} suspended from CPU {}. Budget left: {}",
                       job_id, server_id, cpu_idx,
                       self.state.event_id_to_server_descr.get(&server_id).unwrap().budget);

        cpu_idx
    }

    /// Private helper that puts a job on a cpu its server was not holding: it writes it
    /// in jobs_on_cpu, updates its descriptor and put in priority queue the JobCompletion
    /// of the job and the BudgetExhausted of its server.
    fn schedule_job_on_cpu(&mut self, job_id: u64, cpu_idx: usize, current_time: SimTime) {

        // Get server_id and write on jobs_on_cpu
        let server_id = self.server_id_of_job(job_id);
        self.state.jobs_on_cpu[cpu_idx] = Some(job_id);

        // Update job descriptor and insert JobCompletion
        let completion_time = {
            let job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
            job_desc.scheduled_on_cpu = Some(current_time);
            current_time + job_desc.remaining_time
        };
        self.queue.insert(job_id, completion_time, EventType::JobCompletion).unwrap();

        // Insert BudgetExhausted
        let budget = self.state.event_id_to_server_descr.get(&server_id).unwrap().budget;
        self.queue.insert(server_id, current_time + budget, EventType::BudgetExhausted).unwrap();

        self.stats.cpu_goes_busy(cpu_idx, current_time);

        debug_println!("Job-Id: {} of Server-Id: {} scheduled on CPU {}. Completion-Time: {}, Budget-Exhausted-Time: {}",
                       job_id, server_id, cpu_idx, completion_time, current_time + budget);
    }

    /// Private helper swapping which job of a server runs on the cpu the server is
    /// already holding. Only the JobCompletion is inserted: the server never stopped
    /// running since the BudgetExhausted event is still in priority queue.
    fn switch_job_on_cpu(&mut self, job_id: u64, cpu_idx: usize, current_time: SimTime) {

        self.state.jobs_on_cpu[cpu_idx] = Some(job_id);

        let completion_time = {
            let job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
            job_desc.scheduled_on_cpu = Some(current_time);
            current_time + job_desc.remaining_time
        };
        self.queue.insert(job_id, completion_time, EventType::JobCompletion).unwrap();

        debug_println!("Job-Id: {} took over CPU {} inside its own server. Completion-Time: {}",
                       job_id, cpu_idx, completion_time);
    }

    /// Private helper applying the CBS recharge rule to a server that is about to run
    /// if the budget it holds is greater than its share of the time until deadline we
    /// refresh budget to max and deadline to current time + period
    fn recharge_if_due(&mut self, server_id: u64, current_time: SimTime) {
        let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();

        if server_desc.budget as f64 >= (server_desc.deadline - current_time) as f64 * server_desc.bandwidth {
            server_desc.budget = server_desc.max_budget;
            server_desc.deadline = current_time + server_desc.period;

            debug_println!("Server-Id: {} recharged. Budget: {}, Deadline: {}",
                           server_id, server_desc.budget, server_desc.deadline);
        }
    }

    /// Private helper extracting the next job from the ready queue of a cpu (the job
    /// represents the server it is executing on). We extract job with highest server
    /// priority and check if server has to be recharged (lowering priority) if so we
    /// re-insert job and keep extracting otherwise we return job
    fn pop_best_ready_server(&mut self, cpu_idx: usize, current_time: SimTime) -> Option<u64> {
        loop {
            let (queued_deadline, job_id) = self.state.jobs_ready_queue_on_cpu[cpu_idx].pop_first()?;

            let server_id = self.server_id_of_job(job_id);
            self.recharge_if_due(server_id, current_time);
            let server_deadline = self.state.event_id_to_server_descr.get(&server_id).unwrap().deadline;

            // If the deadline did not move this entry is still the best one
            if server_deadline == queued_deadline {
                return Some(job_id);
            }

            // Otherwise it has to be re-ordered against the others
            self.state.jobs_ready_queue_on_cpu[cpu_idx].insert(job_id, server_deadline);
        }
    }

    /// Private handler for TaskTrigger events.
    /// Creates a new job, decides whether it takes over its server's slot, and if it
    /// does, tries to put it on the cpu its server is pinned to.
    fn handle_task_trigger(&mut self, node: Node<EventType>) {
        let time = node.time;
        let task_id = node.event_id;
        debug_println!("TaskTrigger event extracted. Time: {}, Task-Id: {}", time, task_id);

        // Re-insert the task trigger and sample the parameters of the job it spawns
        let (execution_time, deadline, server_id) = {
            let task_desc = self.state.event_id_to_task_desc.get_mut(&task_id).unwrap();
            debug_println!("TaskDescriptor Info: {:?}", task_desc);

            let delta = task_desc.interarrival_time_rng.sample();
            let execution_time = task_desc.execution_time_rng.sample();
            let deadline = time + task_desc.relative_deadline;
            let server_id = task_desc.server_id;

            self.queue.insert(task_id, time + delta, EventType::TaskTrigger).unwrap();
            (execution_time, deadline, server_id)
        };

        // Create the JobDescriptor for the new job
        let job_id = get_next_id();
        self.state.event_id_to_job_descr.insert(job_id, JobDescriptor {
            scheduled_on_cpu: None,
            remaining_time: execution_time,
            deadline,
            task_id,
        });
        self.stats.register_job(job_id, time, task_id);

        debug_println!("Created JobDescriptor. Job-Id: {}, Execution-Time: {}, Deadline: {}, Server-Id: {}",
                       job_id, execution_time, deadline, server_id);

        // If the new job does not become the executing job of the server you just add it
        // to the ready queue of the server and return
        // Otherwise you have to push the previously executing job to ready queue (if it
        // existed) and keep its id (used later to update stuff)
        let old_current_job = {
            let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();

            match server_desc.current_job {

                // Checking if current job exists and has higher priority
                Some(current_job) if current_job < (deadline, job_id) => {
                    server_desc.ready_queue.push(Reverse((deadline, job_id)));
                    debug_println!("Job-Id: {} queued inside Server-Id: {}", job_id, server_id);
                    return;
                }

                // Branch handling case in which current job exists and is "preempted" and
                // case in which current job does not exist. old_current_job is an option
                old_current_job => {
                    server_desc.current_job = Some((deadline, job_id));
                    if let Some(old_current_job) = old_current_job {
                        server_desc.ready_queue.push(Reverse(old_current_job));
                    }
                    old_current_job
                }
            }
        };

        let affine_cpu = self.affine_cpu_of_server(server_id);

        // If there was a previous running job in the server (Option is not NOne) and if 
        // previous job was scheduled on cpu, it has to be removed (updating all related
        // stuff with private handler), new job has to be inserted (once again updating
        // related stuff with the helper), and return.
        // If there was a previous running job but not scheduled on cpu then you have to 
        // simply update the job_id in the ready queue, and return. You can early return
        // in this case because server priority has not changed so if old job was not 
        // scheduled new job won't be
        if let Some((_, old_job_id)) = old_current_job {
            if self.state.event_id_to_job_descr.get(&old_job_id).unwrap().scheduled_on_cpu.is_some() {
                debug_println!("Job-Id: {} takes over the cpu of Job-Id: {} inside Server-Id: {}",
                               job_id, old_job_id, server_id);
                self.stop_job_execution(old_job_id, time);
                self.switch_job_on_cpu(job_id, affine_cpu, time);
                return;
            }

            let server_deadline = self.state.event_id_to_server_descr.get(&server_id).unwrap().deadline;
            self.state.jobs_ready_queue_on_cpu[affine_cpu].remove(old_job_id);
            self.state.jobs_ready_queue_on_cpu[affine_cpu].insert(job_id, server_deadline);

            debug_println!("Job-Id: {} replaces Job-Id: {} as the CPU {} ready queue entry of Server-Id: {}",
                           job_id, old_job_id, affine_cpu, server_id);
            return;
        }

        // Only a server that had no job at all reaches this point, so this arrival wakes
        // it up and it has to be given its cpu, or queued waiting for it
        self.recharge_if_due(server_id, time);
        let server_deadline = self.state.event_id_to_server_descr.get(&server_id).unwrap().deadline;

        // If its cpu is free take it
        if self.state.jobs_on_cpu[affine_cpu].is_none() {
            self.schedule_job_on_cpu(job_id, affine_cpu, time);
            return;
        }

        // Otherwise preempt the server holding that cpu, if this one is more urgent, and
        // put the preempted server back in the cpu's queue
        let running_job_id = self.state.jobs_on_cpu[affine_cpu].unwrap();
        let running_server_id = self.server_id_of_job(running_job_id);
        let running_server_deadline = self.state.event_id_to_server_descr.get(&running_server_id).unwrap().deadline;

        if server_deadline < running_server_deadline {
            debug_println!("Server-Id: {} preempts Server-Id: {} on CPU {}",
                           server_id, running_server_id, affine_cpu);

            self.suspend_running_job(running_job_id, time);
            let preempted_deadline = self.state.event_id_to_server_descr.get(&running_server_id).unwrap().deadline;
            self.state.jobs_ready_queue_on_cpu[affine_cpu].insert(running_job_id, preempted_deadline);

            self.schedule_job_on_cpu(job_id, affine_cpu, time);
            return;
        }

        // It does not beat the server holding the cpu, so it waits
        self.state.jobs_ready_queue_on_cpu[affine_cpu].insert(job_id, server_deadline);
        debug_println!("Job-Id: {} of Server-Id: {} added to CPU {} ready queue. Server-Deadline: {}",
                       job_id, server_id, affine_cpu, server_deadline);
    }

    /// Private handler for JobCompletion events.
    /// Charges the execution time to the server, removes the job, and gives the cpu
    /// either to the next job of the same server or to the most urgent server waiting
    /// for that same cpu.
    fn handle_job_completion(&mut self, node: Node<EventType>) {
        let time = node.time;
        let job_id = node.event_id;
        debug_println!("JobCompletion event extracted. Time: {}, Job-Id: {}", time, job_id);

        let server_id = self.server_id_of_job(job_id);
        let cpu_idx = self.affine_cpu_of_server(server_id);
        let (scheduled_time, job_deadline) = {
            let job_desc = self.state.event_id_to_job_descr.get(&job_id).unwrap();
            (job_desc.scheduled_on_cpu.unwrap(), job_desc.deadline)
        };

        // Verify job is the one running on that cpu
        assert_eq!(self.state.jobs_on_cpu[cpu_idx], Some(job_id), "JobCompletion for job not on CPU");

        // Charge the run to the server's budget, keeping the event BudgetExhausted
        let exhausted_time = {
            let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();
            let exhausted_time = scheduled_time + server_desc.budget;
            server_desc.budget -= time - scheduled_time;
            exhausted_time
        };

        // Record the completion and remove the job
        self.stats.record_job_completion(job_id, time, job_deadline);
        self.state.event_id_to_job_descr.remove(&job_id);
        debug_println!("Job-Id: {} completed on CPU {}", job_id, cpu_idx);

        // If the server has another job waiting, it sets it as job running on cpu
        let next_own_job = {
            let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();
            server_desc.current_job = server_desc.ready_queue.pop().map(|Reverse(entry)| entry);
            server_desc.current_job
        };
        if let Some((_, next_job_id)) = next_own_job {
            debug_println!("Server-Id: {} keeps CPU {} for its Job-Id: {}", server_id, cpu_idx, next_job_id);
            self.switch_job_on_cpu(next_job_id, cpu_idx, time);
            return;
        }

        // The server went idle, so it stops consuming budget and gives the cpu up
        self.queue.remove(exhausted_time, server_id);
        self.state.jobs_on_cpu[cpu_idx] = None;
        self.stats.cpu_goes_idle(cpu_idx, time);

        // The freed cpu goes to the most urgent server waiting for it
        if let Some(next_job_id) = self.pop_best_ready_server(cpu_idx, time) {
            self.schedule_job_on_cpu(next_job_id, cpu_idx, time);
        } else {
            debug_println!("Ready queue of CPU {} is empty. CPU is now idle.", cpu_idx);
        }
    }

    /// Private handler for BudgetExhausted events.
    /// The server has used up its budget, so job is removed from cpu, budget is
    /// replenished, priority is updated, and the most urgent server of that cpu is run
    fn handle_budget_exhausted(&mut self, node: Node<EventType>) {
        let time = node.time;
        let server_id = node.event_id;
        debug_println!("BudgetExhausted event extracted. Time: {}, Server-Id: {}", time, server_id);

        // Remove running job of server from cpu
        let (_, current_job_id) = self.state.event_id_to_server_descr
            .get(&server_id)
            .unwrap()
            .current_job
            .expect("BudgetExhausted for a server with no current job");
        let cpu_idx = self.suspend_running_job(current_job_id, time);

        // CBS postponement: give the budget back and push the deadline out by one period
        let server_deadline = {
            let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();
            server_desc.budget += server_desc.max_budget;
            server_desc.deadline += server_desc.period;
            debug_println!("Server-Id: {} postponed. Budget: {}, Deadline: {}",
                           server_id, server_desc.budget, server_desc.deadline);
            server_desc.deadline
        };

        // Its deadline moved, so it goes back in its cpu's queue and the cpu is given to
        // whichever server waiting for it is the most urgent now, possibly this same one
        self.state.jobs_ready_queue_on_cpu[cpu_idx].insert(current_job_id, server_deadline);
        let next_job_id = self.pop_best_ready_server(cpu_idx, time).unwrap();
        self.schedule_job_on_cpu(next_job_id, cpu_idx, time);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::prelude::BTreeQueue;
    use crate::base::stats::Deterministic;

    /// A task with deterministic execution time and period, so every run is reproducible
    fn task(start_time: SimTime, relative_deadline: SimTime, execution_time: SimTime, period: SimTime) -> TaskSpec {
        TaskSpec {
            start_time,
            relative_deadline,
            execution_time_rng: Box::new(Deterministic::new(execution_time)),
            interarrival_time_rng: Box::new(Deterministic::new(period)),
            store_response_times: true,
        }
    }

    /// Per-task average response times, ordered by task id, i.e. by registration order
    fn response_times_by_registration_order<Q: EventQueue<EventType, EventPriorityPolicy>>(
        simulator: &CPUSimulatorMQCBS<Q>,
    ) -> Vec<SimTime> {
        let mut task_stats: Vec<_> = simulator.stats().event_id_to_task_stat_descr.iter().collect();
        task_stats.sort_by_key(|&(task_id, _)| task_id);
        task_stats.iter().map(|(_, stat)| stat.avg_response_time).collect()
    }

    #[test]
    fn test_job_runs_untouched_when_budget_is_generous() {
        // Q=10 over T=20 is far more bandwidth than a C=2 job needs, so the server is
        // never throttled and every job completes as soon as it would without a server.
        let mut simulator: CPUSimulatorMQCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMQCBS::new(1)
                .with_end_time(90)
                .add_server(10, 20, vec![task(0, 20, 2, 20)]);
        simulator.simulate();

        // Jobs are released at 0, 20, 40, 60 and 80, each finishing 2 time units later
        assert_eq!(simulator.stats().jobs_done, 5);
        assert_eq!(simulator.stats().jobs_with_missed_deadlines, 0);
        assert_eq!(simulator.stats().num_of_unmet_and_uncompleted_jobs, 0);
        assert_eq!(response_times_by_registration_order(&simulator), vec![2]);
    }

    #[test]
    fn test_job_longer_than_budget_is_throttled_but_completes() {
        // C=5 against a Q=2 budget: the job is interrupted every 2 time units, its
        // server is replenished and postponed, and since nothing else wants its cpu it
        // immediately gets it back, so it still finishes after 5 time units of running.
        let mut simulator: CPUSimulatorMQCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMQCBS::new(1)
                .with_end_time(50)
                .add_server(2, 10, vec![task(0, 50, 5, 100)]);
        simulator.simulate();

        assert_eq!(simulator.stats().jobs_done, 1);
        assert_eq!(simulator.stats().jobs_with_missed_deadlines, 0);
        assert_eq!(response_times_by_registration_order(&simulator), vec![5]);
    }

    #[test]
    fn test_small_bandwidth_server_is_overtaken_on_a_shared_cpu() {
        // With a single cpu both servers are pinned to it, so this is the same race as
        // in mp_cbs: the first one starts, is throttled at t=2, which postpones its
        // deadline and lets the wider server finish at t=6, and only then does the
        // first one get the cpu back and finish at t=8.
        let mut simulator: CPUSimulatorMQCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMQCBS::new(1)
                .with_end_time(50)
                .add_server(2, 10, vec![task(0, 100, 4, 100)])
                .add_server(8, 10, vec![task(0, 100, 4, 100)]);
        simulator.simulate();

        assert_eq!(simulator.stats().jobs_done, 2);
        assert_eq!(simulator.stats().jobs_with_missed_deadlines, 0);
        assert_eq!(response_times_by_registration_order(&simulator), vec![8, 6]);
    }

    #[test]
    fn test_servers_are_partitioned_over_the_cpus() {
        // Two servers and two cpus: the second one is pinned to the cpu left empty by
        // the first, so neither has to wait and the narrow budget never bites. This is
        // the partitioning paying off, the same taskset shares one cpu in the test above.
        let mut simulator: CPUSimulatorMQCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMQCBS::new(2)
                .with_end_time(50)
                .add_server(2, 10, vec![task(0, 100, 4, 100)])
                .add_server(8, 10, vec![task(0, 100, 4, 100)]);
        simulator.simulate();

        assert_eq!(simulator.stats().jobs_done, 2);
        assert_eq!(response_times_by_registration_order(&simulator), vec![4, 4]);
    }

    #[test]
    fn test_two_tasks_share_one_server() {
        // One server serving two tasks: the C=1 job released at t=0 with the earlier
        // deadline runs first, the C=1 job behind it runs straight after on the same
        // cpu without the server ever giving it up, so its BudgetExhausted is never
        // re-armed and still fires at the right instant.
        let mut simulator: CPUSimulatorMQCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMQCBS::new(1)
                .with_end_time(50)
                .add_server(10, 20, vec![task(0, 10, 1, 100), task(0, 20, 1, 100)]);
        simulator.simulate();

        assert_eq!(simulator.stats().jobs_done, 2);
        assert_eq!(simulator.stats().jobs_with_missed_deadlines, 0);
        // The earlier-deadline job finishes at 1, the other waits for it and finishes at 2
        assert_eq!(response_times_by_registration_order(&simulator), vec![1, 2]);
    }

    #[test]
    fn test_least_loaded_cpu_gets_the_next_server() {
        // Bandwidths 0.8, 0.1 and 0.1 over two cpus: the first server takes cpu 0, the
        // second goes to the empty cpu 1, and the third joins it there since cpu 1 is
        // still the least loaded. The two sharing cpu 1 are released together and have
        // to take turns on it even though they only ask for 0.2 of it between them, so
        // the one that loses the tie runs second and answers in 2 instead of 1.
        let mut simulator: CPUSimulatorMQCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMQCBS::new(2)
                .with_end_time(50)
                .add_server(8, 10, vec![task(0, 100, 4, 100)])
                .add_server(1, 10, vec![task(0, 100, 1, 100)])
                .add_server(1, 10, vec![task(0, 100, 1, 100)]);
        simulator.simulate();

        assert_eq!(simulator.stats().jobs_done, 3);
        assert_eq!(response_times_by_registration_order(&simulator), vec![4, 1, 2]);
    }
}
