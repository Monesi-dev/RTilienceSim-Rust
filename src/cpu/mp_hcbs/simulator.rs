//! # Hard CBS Preemptive CPU Scheduler Simulator (Multi-Processor, Global Queue)
//!
//! This module simulates hard Constant Bandwidth Server scheduling on M processors with
//! one global queue of servers. The scheduling is on 2 levels, exactly as in `mp_cbs`:
//!
//! - **Between servers**: servers compete for the M CPUs using server deadline as
//!   priority, basically EDF between servers. A server deadline can change when its
//!   budget is emptied and refilled.
//! - **Inside a server**: When a server is scheduled only one of its jobs is executed,
//!   it is chosen using an internal priority, which is the job's deadline
//!
//! The rules are:
//! - On being dispatched after not running, if `c >= (d - t) * U` the old budget would
//!   give the server more than its share, so it restarts its window: `c = Q, d = t + T`.
//! - On running out of budget the server goes **dormant** instead of being postponed: it
//!   leaves the cpu, stays out of the queue, and a BudgetReplenished event armed for its
//!   deadline later gives it `c = Q, d += T` and lets it compete again.
//!
//! That second rule is the whole difference from `mp_cbs`. There a throttled server is
//! handed its budget back immediately and only falls behind in the ordering, so it keeps
//! running whenever nothing more urgent wants the cpu and can exceed its share on an
//! empty machine. Here it is made to sit out the rest of the period it already spent, so
//! it can never take more than Q out of any T no matter how idle the other servers are.

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
pub struct CPUSimulatorMPHCBS<Q: EventQueue<EventType, EventPriorityPolicy>> {
    state: GlobalState,
    queue: Q,
    stats: Stats,
}

impl<Q: EventQueue<EventType, EventPriorityPolicy>> CPUSimulatorMPHCBS<Q> {

    pub fn new(num_cpus: usize) -> Self {
        CPUSimulatorMPHCBS {
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
    /// `period` (T), and registers every task in `tasks` as served by it. All of these
    /// tasks share its budget.
    pub fn add_server(mut self, max_budget: SimTime, period: SimTime, tasks: Vec<TaskSpec>) -> Self {

        // Create the server descriptor and its event_id -> server_descr mapping
        let server_id = get_next_id();
        self.state.event_id_to_server_descr.insert(
            server_id,
            ServerDescriptor::new(max_budget, period),
        );
        debug_println!("Added server Server-Id: {}, Q={}, T={}", server_id, max_budget, period);

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
                EventType::BudgetReplenished => self.handle_budget_replenished(node),
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

    /// Private helper that puts a job on a cpu: it writes it in jobs_on_cpu, updates its
    /// descriptor and put in priority queue the JobCompletion of the job and the 
    /// BudgetExhausted of its server.
    fn schedule_job_on_cpu(&mut self, job_id: u64, cpu_idx: usize, current_time: SimTime) {

        // Get server_id and write on jobs_on_cpu
        let server_id = self.server_id_of_job(job_id);
        self.state.jobs_on_cpu[cpu_idx] = Some(job_id);

        // Update job descriptor and insert JobCompletion
        let completion_time = {
            let job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
            job_desc.scheduled_on_cpu = Some((current_time, cpu_idx));
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

    /// Private helper that takes a running job off its cpu without finishing it: it
    /// charges the time it ran for to job's remaining time and server's budget, deletes 
    /// JobCompletion and BudgetExhausted events, and frees the cpu.
    /// Returns the index of the cpu that was freed.
    fn suspend_running_job(&mut self, job_id: u64, current_time: SimTime) -> usize {

        let server_id = self.server_id_of_job(job_id);

        // Charge the run time to job and compute the completion time so JobCompletion
        // event can be removed
        let (scheduled_time, cpu_idx, completion_time) = {
            let job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
            let (scheduled_time, cpu_idx) = job_desc.scheduled_on_cpu.unwrap();
            let completion_time = scheduled_time + job_desc.remaining_time;
            job_desc.remaining_time -= current_time - scheduled_time;
            job_desc.scheduled_on_cpu = None;
            (scheduled_time, cpu_idx, completion_time)
        };
        self.queue.remove(completion_time, job_id);

        // Charge the same time to server's budget and delete BudgetExhausted
        let exhausted_time = {
            let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();
            let exhausted_time = scheduled_time + server_desc.budget;
            server_desc.budget -= current_time - scheduled_time;
            exhausted_time
        };
        self.queue.remove(exhausted_time, server_id);

        // Sets jobs_on_cpu to None
        self.state.jobs_on_cpu[cpu_idx] = None;
        self.stats.cpu_goes_idle(cpu_idx, current_time);

        debug_println!("Job-Id: {} of Server-Id: {} suspended from CPU {}. Budget left: {}",
                       job_id, server_id, cpu_idx,
                       self.state.event_id_to_server_descr.get(&server_id).unwrap().budget);

        cpu_idx
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

    /// Private helper extracting the next job from ready queue (the job represents the
    /// server it is executing on). We extract job with highest server priority and check
    /// if server has to be recharged (lowering priority) if so we re-insert job and keep
    /// extracting otherwise we return job
    fn pop_best_ready_server(&mut self, current_time: SimTime) -> Option<u64> {
        loop {
            let (queued_deadline, job_id) = self.state.jobs_ready_queue.pop_first()?;

            let server_id = self.server_id_of_job(job_id);
            self.recharge_if_due(server_id, current_time);
            let server_deadline = self.state.event_id_to_server_descr.get(&server_id).unwrap().deadline;

            // If the deadline did not move this entry is still the best one
            if server_deadline == queued_deadline {
                return Some(job_id);
            }

            // Otherwise it has to be re-ordered against the others
            self.state.jobs_ready_queue.insert(job_id, server_deadline);
        }
    }

    /// Private helper placing a server that is ready to run: it takes a free cpu if
    /// there is one, otherwise it preempts the least urgent running server if it beats
    /// it, otherwise it waits in the global queue. Used both when an arriving job wakes
    /// an idle server and when a dormant one is replenished, which are the two ways a
    /// server can go from not running to wanting a cpu.
    fn place_runnable_server(&mut self, job_id: u64, server_deadline: SimTime, current_time: SimTime) {

        // If there is an idle cpu take it
        if let Some(idle_cpu) = self.find_idle_cpu() {
            self.schedule_job_on_cpu(job_id, idle_cpu, current_time);
            return;
        }

        // Otherwise preempt the least urgent running server, if this one beats it, and
        // put the preempted server back in the global queue
        let (victim_cpu, latest_deadline) = self.find_cpu_with_latest_server_deadline().unwrap();
        if server_deadline < latest_deadline {
            let victim_job_id = self.state.jobs_on_cpu[victim_cpu].unwrap();
            debug_println!("Job-Id: {} preempts Job-Id: {} on CPU {}", job_id, victim_job_id, victim_cpu);

            self.suspend_running_job(victim_job_id, current_time);
            let victim_server_id = self.server_id_of_job(victim_job_id);
            let victim_deadline = self.state.event_id_to_server_descr.get(&victim_server_id).unwrap().deadline;
            self.state.jobs_ready_queue.insert(victim_job_id, victim_deadline);

            self.schedule_job_on_cpu(job_id, victim_cpu, current_time);
            return;
        }

        // It does not beat anyone, so it waits
        self.state.jobs_ready_queue.insert(job_id, server_deadline);
        debug_println!("Job-Id: {} added to global ready queue. Server-Deadline: {}",
                       job_id, server_deadline);
    }

    /// Private helper that finds an idle cpu, returns its index if there is one
    fn find_idle_cpu(&self) -> Option<usize> {
        for (cpu_idx, job_id_opt) in self.state.jobs_on_cpu.iter().enumerate() {
            if job_id_opt.is_none() {
                return Some(cpu_idx);
            }
        }
        None
    }

    /// Private helper that finds the cpu running the server with the latest deadline,
    /// i.e. the least urgent one, which is the one to preempt
    fn find_cpu_with_latest_server_deadline(&self) -> Option<(usize, SimTime)> {
        let mut latest_cpu = None;
        let mut latest_deadline = SimTime::MIN;

        for (cpu_idx, job_id_opt) in self.state.jobs_on_cpu.iter().enumerate() {
            let job_id = job_id_opt.unwrap();
            let server_id = self.server_id_of_job(job_id);
            let server_deadline = self.state.event_id_to_server_descr.get(&server_id).unwrap().deadline;
            if server_deadline > latest_deadline {
                latest_deadline = server_deadline;
                latest_cpu = Some((cpu_idx, latest_deadline));
            }
        }

        latest_cpu
    }

    /// Private handler for TaskTrigger events.
    /// Creates a new job, decides whether it takes over its server's slot, and if it
    /// does, finds the new job a cpu.
    fn handle_task_trigger(&mut self, node: Node<EventType>) {
        let time = node.time;
        let task_id = node.event_id;
        debug_println!("TaskTrigger event extracted. Time: {}, Task-Id: {}", time, task_id);

        // Re-inser the task trigger and sample the parameters of the job it spawns
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
                // cas ein which current job does not exist. old_current_job is an option
                old_current_job => {
                    server_desc.current_job = Some((deadline, job_id));
                    if let Some(old_current_job) = old_current_job {
                        server_desc.ready_queue.push(Reverse(old_current_job));
                    }
                    old_current_job
                }
            }
        };

        // A dormant server is on no cpu and in no queue, so settling which of its jobs
        // comes first is all this arrival can do: whatever is current when the server is
        // replenished is what gets dispatched then
        if !self.state.event_id_to_server_descr.get(&server_id).unwrap().active {
            debug_println!("Server-Id: {} is waiting to be replenished, Job-Id: {} stays inside it",
                           server_id, job_id);
            return;
        }

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
                let cpu_idx = self.suspend_running_job(old_job_id, time);
                self.schedule_job_on_cpu(job_id, cpu_idx, time);
                return;
            }

            let server_deadline = self.state.event_id_to_server_descr.get(&server_id).unwrap().deadline;
            self.state.jobs_ready_queue.remove(old_job_id);
            self.state.jobs_ready_queue.insert(job_id, server_deadline);

            debug_println!("Job-Id: {} replaces Job-Id: {} as the global ready queue entry of Server-Id: {}",
                           job_id, old_job_id, server_id);
            return;
        }

        // Only a server that had no job at all reaches this point, so we check if server
        // has to be recharged and then we get its deadline
        self.recharge_if_due(server_id, time);
        let server_deadline = self.state.event_id_to_server_descr.get(&server_id).unwrap().deadline;

        // The server is awake and wants a cpu, so it goes through the usual placement
        self.place_runnable_server(job_id, server_deadline, time);
    }

    /// Private handler for JobCompletion events.
    /// Charges the executio time to the server, removes the job, and gives the cpu 
    /// either to the next job of the same server or to the most urgent waiting server.
    fn handle_job_completion(&mut self, node: Node<EventType>) {
        let time = node.time;
        let job_id = node.event_id;
        debug_println!("JobCompletion event extracted. Time: {}, Job-Id: {}", time, job_id);

        let server_id = self.server_id_of_job(job_id);
        let (scheduled_time, cpu_idx, job_deadline) = {
            let job_desc = self.state.event_id_to_job_descr.get(&job_id).unwrap();
            let (scheduled_time, cpu_idx) = job_desc.scheduled_on_cpu.unwrap();
            (scheduled_time, cpu_idx, job_desc.deadline)
        };

        // Verify job is the one running on that cpu
        assert_eq!(self.state.jobs_on_cpu[cpu_idx], Some(job_id), "JobCompletion for job not on CPU");

        // Charge the run to the server's budget and delete the BudgetExhausted 
        let exhausted_time = {
            let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();
            let exhausted_time = scheduled_time + server_desc.budget;
            server_desc.budget -= time - scheduled_time;
            exhausted_time
        };
        self.queue.remove(exhausted_time, server_id);

        // Record the completion and remove the job
        self.stats.record_job_completion(job_id, time, job_deadline);
        self.state.event_id_to_job_descr.remove(&job_id);
        self.state.jobs_on_cpu[cpu_idx] = None;
        self.stats.cpu_goes_idle(cpu_idx, time);
        debug_println!("Job-Id: {} completed and removed from CPU {}", job_id, cpu_idx);

        // If the server has another job of its own waiting, it keeps the cpu for it
        let next_own_job = {
            let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();
            server_desc.current_job = server_desc.ready_queue.pop().map(|Reverse(entry)| entry);
            server_desc.current_job
        };
        if let Some((_, next_job_id)) = next_own_job {
            debug_println!("Server-Id: {} keeps CPU {} for its Job-Id: {}", server_id, cpu_idx, next_job_id);
            self.schedule_job_on_cpu(next_job_id, cpu_idx, time);
            return;
        }

        // The server went idle, so the freed cpu goes to the most urgent waiting server
        if let Some(next_job_id) = self.pop_best_ready_server(time) {
            self.schedule_job_on_cpu(next_job_id, cpu_idx, time);
        } else {
            debug_println!("Global ready queue is empty. CPU {} is now idle.", cpu_idx);
        }
    }

    /// Private handler for BudgetExhausted events.
    /// The server has used up its budget, so its job is taken off the cpu and the server
    /// goes dormant until its deadline, when BudgetReplenished wakes it up. Unlike the
    /// soft CBS of `mp_cbs`, server does not go back in queue
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

        // Put the server to sleep until its deadline, by setting active to false and
        // inserting BudgetReplenished in queue
        let replenish_time = {
            let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();
            server_desc.active = false;
            server_desc.deadline
        };
        self.queue.insert(server_id, replenish_time, EventType::BudgetReplenished).unwrap();
        debug_println!("Server-Id: {} went dormant, will be replenished at {}", server_id, replenish_time);

        // The cpu it was holding goes to the most urgent server still awake, if any
        if let Some(next_job_id) = self.pop_best_ready_server(time) {
            self.schedule_job_on_cpu(next_job_id, cpu_idx, time);
        } else {
            debug_println!("Global ready queue is empty. CPU {} is now idle.", cpu_idx);
        }
    }

    /// Private handler for BudgetReplenished events.
    /// The dormant server has reached its deadline, so it gets a full budget and the
    /// next period and goes back to competing for a cpu. 
    fn handle_budget_replenished(&mut self, node: Node<EventType>) {
        let time = node.time;
        let server_id = node.event_id;
        debug_println!("BudgetReplenished event extracted. Time: {}, Server-Id: {}", time, server_id);

        // Full budget, next period, and the server can be scheduled again
        let server_deadline = {
            let server_desc = self.state.event_id_to_server_descr.get_mut(&server_id).unwrap();
            server_desc.budget = server_desc.max_budget;
            server_desc.deadline += server_desc.period;
            server_desc.active = true;
            debug_println!("Server-Id: {} replenished. Budget: {}, Deadline: {}",
                           server_id, server_desc.budget, server_desc.deadline);
            server_desc.deadline
        };

        // A server only ever goes dormant while it is running, and nothing can take that
        // job away while it sleeps, so it always has one to put forward here
        let (_, current_job_id) = self.state.event_id_to_server_descr
            .get(&server_id)
            .unwrap()
            .current_job
            .expect("BudgetReplenished for a server with no current job");

        self.place_runnable_server(current_job_id, server_deadline, time);
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
        simulator: &CPUSimulatorMPHCBS<Q>,
    ) -> Vec<SimTime> {
        let mut task_stats: Vec<_> = simulator.stats().event_id_to_task_stat_descr.iter().collect();
        task_stats.sort_by_key(|&(task_id, _)| task_id);
        task_stats.iter().map(|(_, stat)| stat.avg_response_time).collect()
    }

    #[test]
    fn test_job_runs_untouched_when_budget_is_generous() {
        // Q=10 over T=20 is far more bandwidth than a C=2 job needs, so the server is
        // never throttled and every job completes as soon as it would without a server.
        let mut simulator: CPUSimulatorMPHCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMPHCBS::new(1)
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
    fn test_budget_is_capped_even_on_an_idle_machine() {
        // This is the whole point of the hard variant. C=5 against a Q=2 out of T=10
        // reservation, with nothing else on the machine: the job runs 2, sits out the
        // rest of the period, runs 2 more, sits out again, and finishes its last unit
        // in the third period, so it answers at 21 rather than at 5. mp_cbs, which hands
        // the budget straight back and only moves the deadline, finishes this at 5 and
        // so lets the server take 5 out of the first 10 time units instead of 2.
        let mut simulator: CPUSimulatorMPHCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMPHCBS::new(1)
                .with_end_time(50)
                .add_server(2, 10, vec![task(0, 50, 5, 100)]);
        simulator.simulate();

        assert_eq!(simulator.stats().jobs_done, 1);
        assert_eq!(simulator.stats().jobs_with_missed_deadlines, 0);
        assert_eq!(response_times_by_registration_order(&simulator), vec![21]);
    }

    #[test]
    fn test_small_bandwidth_server_is_overtaken_by_large_one() {
        // Both servers release a C=4 job at time 0 on a single cpu, but the first one
        // only reserves 2 out of every 10 time units while the second reserves 8. The
        // first one starts (it is registered first, so it wins the tie on equal
        // deadlines) and is throttled at t=2, which puts it to sleep until t=10 and lets
        // the wider server run to completion at t=6. The cpu then sits idle until the
        // narrow server wakes up and finishes its remaining 2 units at t=12 - under the
        // soft CBS of mp_cbs it would have taken the free cpu straight away and finished
        // at t=8 instead.
        let mut simulator: CPUSimulatorMPHCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMPHCBS::new(1)
                .with_end_time(50)
                .add_server(2, 10, vec![task(0, 100, 4, 100)])
                .add_server(8, 10, vec![task(0, 100, 4, 100)]);
        simulator.simulate();

        assert_eq!(simulator.stats().jobs_done, 2);
        assert_eq!(simulator.stats().jobs_with_missed_deadlines, 0);
        assert_eq!(response_times_by_registration_order(&simulator), vec![12, 6]);
    }

    #[test]
    fn test_job_arriving_at_a_dormant_server_waits_for_the_replenishment() {
        // The first task burns the whole Q=2 budget by t=2 and its server goes dormant
        // until t=10. The second task of the same server is released at t=3, while the
        // server is asleep: it is more urgent (deadline 23 against 50) so it takes the
        // server's slot, but it cannot be put on a cpu, not even an idle one, until the
        // server wakes up and puts it forward at t=10.
        let mut simulator: CPUSimulatorMPHCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMPHCBS::new(1)
                .with_end_time(50)
                .add_server(2, 10, vec![task(0, 50, 5, 100), task(3, 20, 1, 100)]);
        simulator.simulate();

        assert_eq!(simulator.stats().jobs_done, 2);
        assert_eq!(simulator.stats().jobs_with_missed_deadlines, 0);
        // The displaced C=5 job only finishes at 22, the C=1 one that overtook it inside
        // the dormant server runs first at t=10 and answers 8 time units after arriving
        assert_eq!(response_times_by_registration_order(&simulator), vec![22, 8]);
    }

    #[test]
    fn test_two_tasks_share_one_server() {
        // One server serving two tasks: the C=1 job released at t=0 with the earlier
        // deadline runs first, the C=1 job behind it runs straight after on the same
        // cpu without the server ever giving it up.
        let mut simulator: CPUSimulatorMPHCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMPHCBS::new(1)
                .with_end_time(50)
                .add_server(10, 20, vec![task(0, 10, 1, 100), task(0, 20, 1, 100)]);
        simulator.simulate();

        assert_eq!(simulator.stats().jobs_done, 2);
        assert_eq!(simulator.stats().jobs_with_missed_deadlines, 0);
        // The earlier-deadline job finishes at 1, the other waits for it and finishes at 2
        assert_eq!(response_times_by_registration_order(&simulator), vec![1, 2]);
    }

    #[test]
    fn test_jobs_spread_over_both_cpus() {
        // Two servers, two cpus: neither has to wait for the other
        let mut simulator: CPUSimulatorMPHCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
            CPUSimulatorMPHCBS::new(2)
                .with_end_time(50)
                .add_server(10, 20, vec![task(0, 100, 4, 100)])
                .add_server(10, 20, vec![task(0, 100, 4, 100)]);
        simulator.simulate();

        assert_eq!(simulator.stats().jobs_done, 2);
        assert_eq!(response_times_by_registration_order(&simulator), vec![4, 4]);
    }
}
